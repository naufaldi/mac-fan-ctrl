mod alerts;
mod apple_silicon_sensors;
mod commands;
mod fan_control;
mod log;
mod power_monitor;
mod power_presets;
mod presets;
mod smc;
mod smc_writer;
mod tray;

use std::thread;
use std::time::{Duration, Instant};

use tauri::{Emitter, Manager};

use commands::AppState;
use log::{debug_log, warn_log};
use power_monitor::PowerSource;
use smc::FanMode;

fn bootstrap_menu_bar(app: &mut tauri::App) {
    debug_log!("[mac-fan-ctrl] Starting shell bootstrap");

    match tauri::menu::Menu::default(app.handle()) {
        Ok(menu) => match app.set_menu(menu) {
            Ok(_) => debug_log!("[mac-fan-ctrl] Menu bar bootstrapped successfully"),
            Err(error) => {
                debug_log!(
                    "[mac-fan-ctrl] Menu bar setup failed, continuing without custom menu: {error}"
                );
            }
        },
        Err(error) => {
            debug_log!(
                "[mac-fan-ctrl] Menu baseline not available, continuing with defaults: {error}"
            );
        }
    }

    debug_log!("[mac-fan-ctrl] Shell bootstrap complete");
}

fn run_fan_control_tick(app_handle: &tauri::AppHandle, sensor_data: &mut smc::SensorData) {
    let state = app_handle.state::<AppState>();
    if let Ok(writer_guard) = state.smc_writer.lock() {
        if let Some(writer) = writer_guard.as_deref() {
            if let Ok(mut control) = state.fan_control.lock() {
                if let Err(error) = control.tick(&sensor_data.details, &sensor_data.fans, writer) {
                    warn_log!("[mac-fan-ctrl] Fan control tick failed: {error}");
                }
                // Overlay configured target/mode onto raw SMC data so the
                // frontend displays the user's settings, not stale readbacks.
                control.overlay_configs(&mut sensor_data.fans);
            }
        }
    };
}

fn run_startup_diagnostics(app_handle: &tauri::AppHandle) {
    let state = app_handle.state::<AppState>();
    let uid = unsafe { libc::getuid() };
    let euid = unsafe { libc::geteuid() };
    debug_log!("\n[mac-fan-ctrl] === STARTUP DIAGNOSTICS ===");
    debug_log!("[mac-fan-ctrl] UID={uid} EUID={euid} running_as_root={}", euid == 0);

    if let Ok(writer_guard) = state.smc_writer.lock() {
        match writer_guard.as_deref() {
            Some(writer) => {
                debug_log!("[mac-fan-ctrl] SMC Writer: AVAILABLE");
                writer.diagnose_fan_control();
            }
            None => {
                debug_log!("[mac-fan-ctrl] SMC Writer: NOT AVAILABLE — fan control will not work");
                debug_log!("[mac-fan-ctrl] Likely cause: app not running as root (EUID={euid})");
            }
        }
    }
    debug_log!("[mac-fan-ctrl] === END STARTUP DIAGNOSTICS ===\n");
}

fn restore_active_preset(app_handle: &tauri::AppHandle) {
    let state = app_handle.state::<AppState>();

    let preset_name = {
        let store = match state.preset_store.lock() {
            Ok(s) => s,
            Err(_) => return,
        };
        match store.active_preset.as_deref() {
            Some("Automatic") | None => return, // Auto is the default — nothing to apply
            Some(name) => name.to_string(),
        }
    };

    debug_log!("[mac-fan-ctrl] Restoring saved preset: {preset_name}");

    let writer_guard = match state.smc_writer.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    let Some(writer) = writer_guard.as_deref() else {
        debug_log!("[mac-fan-ctrl] Cannot restore preset — SMC writer not available");
        return;
    };

    let mut service = smc::SensorService::new();
    let fans = service.read_fans_only();
    let fan_indices: Vec<u8> = fans.iter().map(|f| f.index).collect();
    let fan_maxes: std::collections::HashMap<u8, f32> =
        fans.iter().map(|f| (f.index, f.max)).collect();

    let store = match state.preset_store.lock() {
        Ok(s) => s,
        Err(_) => return,
    };
    let all = presets::all_presets(&store, &fan_indices, &fan_maxes);
    let Some(preset) = all.iter().find(|p| p.name == preset_name) else {
        debug_log!("[mac-fan-ctrl] Saved preset '{preset_name}' not found — skipping restore");
        return;
    };

    let mut fan_control = match state.fan_control.lock() {
        Ok(c) => c,
        Err(_) => return,
    };

    for (fan_index, config) in &preset.configs {
        if let Err(error) = fan_control.set_config(*fan_index, config.clone(), &fans, writer) {
            warn_log!("[mac-fan-ctrl] Failed to restore fan {fan_index}: {error}");
        }
    }

    debug_log!("[mac-fan-ctrl] Preset '{preset_name}' restored successfully");
}

fn check_temperature_alerts(
    app_handle: &tauri::AppHandle,
    sensor_data: &smc::SensorData,
    last_alert_time: &mut Option<Instant>,
) {
    let state = app_handle.state::<AppState>();
    let config = match state.alert_config.lock() {
        Ok(c) => c.clone(),
        Err(_) => return,
    };

    if !config.enabled {
        return;
    }

    let cpu_temp = sensor_data
        .summary
        .cpu_package
        .as_ref()
        .and_then(|s| s.value);

    let Some(temp) = cpu_temp else {
        return;
    };

    let is_over_threshold = temp >= config.cpu_threshold;

    if !is_over_threshold {
        return;
    }

    // Check cooldown
    let cooldown = Duration::from_secs(config.cooldown_secs);
    if let Some(last) = last_alert_time {
        if last.elapsed() < cooldown {
            return;
        }
    }

    warn_log!(
        "[mac-fan-ctrl] ALERT: CPU temp {temp:.1}°C >= {:.1}°C threshold",
        config.cpu_threshold
    );

    // Fire native notification
    use tauri_plugin_notification::NotificationExt;
    let _ = app_handle
        .notification()
        .builder()
        .title("High Temperature Warning")
        .body(format!(
            "CPU temperature is {temp:.0}°C (threshold: {:.0}°C)",
            config.cpu_threshold
        ))
        .show();

    *last_alert_time = Some(Instant::now());
}

/// Shared helper: apply a named preset to all fans.
/// Used by both tray preset selection and power source auto-switch.
pub fn apply_preset_by_name(app_handle: &tauri::AppHandle, preset_name: &str) {
    let state = app_handle.state::<AppState>();

    let writer_guard = match state.smc_writer.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    let Some(writer) = writer_guard.as_deref() else {
        return;
    };

    let mut service = smc::SensorService::new();
    let fans = service.read_fans_only();
    let fan_indices: Vec<u8> = fans.iter().map(|f| f.index).collect();
    let fan_maxes: std::collections::HashMap<u8, f32> =
        fans.iter().map(|f| (f.index, f.max)).collect();

    let mut store = match state.preset_store.lock() {
        Ok(s) => s,
        Err(_) => return,
    };
    let all = presets::all_presets(&store, &fan_indices, &fan_maxes);
    let Some(preset) = all.into_iter().find(|p| p.name == preset_name) else {
        warn_log!("[mac-fan-ctrl] Preset '{preset_name}' not found — cannot apply");
        return;
    };

    let mut control = match state.fan_control.lock() {
        Ok(c) => c,
        Err(_) => return,
    };

    control.restore_all_auto(writer);
    for (fan_index, config) in &preset.configs {
        let _ = control.set_config(*fan_index, config.clone(), &fans, writer);
    }

    store.active_preset = Some(preset_name.to_string());
    let _ = presets::save_preset_store(&store);
    debug_log!("[mac-fan-ctrl] Applied preset '{preset_name}'");
}

fn check_power_source_change(
    app_handle: &tauri::AppHandle,
    last_power_source: &mut PowerSource,
) {
    let current = power_monitor::current_power_source();
    if current == *last_power_source {
        return;
    }

    let previous = *last_power_source;
    *last_power_source = current;

    debug_log!("[mac-fan-ctrl] Power source changed: {previous} -> {current}");

    // Update stored state
    let state = app_handle.state::<AppState>();
    if let Ok(mut stored) = state.current_power_source.lock() {
        *stored = current;
    }

    // Emit event to frontend
    let _ = app_handle.emit("power_source_changed", current);

    // Auto-apply configured preset
    let config = match state.power_preset_config.lock() {
        Ok(c) => c.clone(),
        Err(_) => return,
    };

    if !config.enabled {
        return;
    }

    let preset_name = match current {
        PowerSource::Ac => config.ac_preset,
        PowerSource::Battery => config.battery_preset,
        PowerSource::Unknown => None,
    };

    if let Some(name) = preset_name {
        debug_log!("[mac-fan-ctrl] Auto-switching to preset '{name}' for {current}");
        apply_preset_by_name(app_handle, &name);
    }
}

fn start_sensor_stream(app_handle: tauri::AppHandle) {
    thread::spawn(move || {
        let mut service = smc::SensorService::new();
        let fast_interval = Duration::from_millis(1000);
        let full_read_every = 3; // Do a full sensor read every Nth cycle
        let mut cycle_count: u32 = 0;
        let mut last_full_data: Option<smc::SensorData> = None;
        let mut last_alert_time: Option<Instant> = None;
        let mut last_power_source = power_monitor::current_power_source();

        loop {
            let cycle_start = Instant::now();
            let is_full_cycle = cycle_count % full_read_every == 0;

            if is_full_cycle {
                // Full read: temperatures + fans (slow due to all_data() scan)
                match service.read_all_sensors() {
                    Ok(mut sensor_data) => {
                        run_fan_control_tick(&app_handle, &mut sensor_data);
                        check_temperature_alerts(&app_handle, &sensor_data, &mut last_alert_time);
                        check_power_source_change(&app_handle, &mut last_power_source);

                        debug_log!("[stream] cycle={cycle_count} emit(full)");
                        if let Err(error) =
                            app_handle.emit(commands::SENSOR_UPDATE_EVENT, &sensor_data)
                        {
                            warn_log!("[mac-fan-ctrl] Failed to emit sensor_update event: {error}");
                        }

                        // Full tray update: temperature title + menu rebuild
                        debug_log!("[stream] cycle={cycle_count} update_tray_title+menu");
                        tray::update_tray_title(&app_handle, &sensor_data);
                        tray::update_tray_menu(&app_handle, &sensor_data);

                        last_full_data = Some(sensor_data);
                    }
                    Err(error) => {
                        warn_log!("[mac-fan-ctrl] Sensor stream read failed: {error}");
                        service = smc::SensorService::new();
                    }
                }
            } else if let Some(ref mut cached) = last_full_data {
                // Fast path: only re-read fan data (~10 key reads, <50ms)
                let fresh_fans = service.read_fans_only();
                if !fresh_fans.is_empty() {
                    cached.fans = fresh_fans;
                }

                run_fan_control_tick(&app_handle, cached);

                debug_log!("[stream] cycle={cycle_count} emit(fast)");
                if let Err(error) = app_handle.emit(commands::SENSOR_UPDATE_EVENT, &*cached) {
                    warn_log!("[mac-fan-ctrl] Failed to emit sensor_update event: {error}");
                }

                // Fast tray update: temperature title only
                debug_log!("[stream] cycle={cycle_count} update_tray_title");
                tray::update_tray_title(&app_handle, cached);
            }

            cycle_count = cycle_count.wrapping_add(1);

            let elapsed = cycle_start.elapsed();
            if let Some(remaining) = fast_interval.checked_sub(elapsed) {
                thread::sleep(remaining);
            }
        }
    });
}

fn restore_fans(app_handle: &tauri::AppHandle) {
    let state = app_handle.state::<AppState>();
    let Ok(writer_guard) = state.smc_writer.lock() else {
        return;
    };
    let Some(writer) = writer_guard.as_deref() else {
        return;
    };
    let Ok(mut control) = state.fan_control.lock() else {
        return;
    };
    warn_log!("[mac-fan-ctrl] Restoring all fans to Auto");
    control.restore_all_auto(writer);
}

fn recover_orphaned_fan_modes(app_handle: &tauri::AppHandle) {
    let state = app_handle.state::<AppState>();

    // Only recover if no active session configs exist (i.e. fresh startup)
    if let Ok(control) = state.fan_control.lock() {
        if !control.configs().is_empty() {
            return;
        }
    }

    let Ok(writer_guard) = state.smc_writer.lock() else {
        return;
    };
    let Some(writer) = writer_guard.as_deref() else {
        return;
    };

    let mut service = smc::SensorService::new();
    let fans = service.read_fans_only();

    let orphaned: Vec<u8> = fans
        .iter()
        .filter(|f| f.mode == FanMode::Forced)
        .map(|f| f.index)
        .collect();

    if orphaned.is_empty() {
        return;
    }

    warn_log!(
        "[mac-fan-ctrl] RECOVERY: Found {} orphaned fan(s) in Forced mode: {:?}",
        orphaned.len(),
        orphaned
    );

    for fan_index in &orphaned {
        match writer.set_fan_auto(*fan_index) {
            Ok(()) => warn_log!("[mac-fan-ctrl] RECOVERY: Fan {fan_index} restored to Auto"),
            Err(e) => warn_log!("[mac-fan-ctrl] RECOVERY: Failed to restore fan {fan_index}: {e}"),
        }
    }

    let _ = writer.lock_fan_control();
    warn_log!("[mac-fan-ctrl] RECOVERY: Thermal enforcement re-locked");
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState::new())
        .setup(|app| {
            bootstrap_menu_bar(app);
            run_startup_diagnostics(app.handle());
            recover_orphaned_fan_modes(app.handle());
            restore_active_preset(app.handle());

            // Initialize menu bar tray icon
            match tray::setup_tray(app) {
                Ok(tray_icon) => {
                    app.manage(commands::TrayHandle(tray_icon));
                }
                Err(e) => {
                    warn_log!("[mac-fan-ctrl] Tray setup failed: {e}");
                }
            }

            // Hide Dock icon — app lives in the menu bar
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Bring window to front after Accessory policy change
            // (macOS deactivates the app when removing its dock icon)
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }

            // Register signal handler for SIGTERM/SIGINT
            let signal_handle = app.handle().clone();
            ctrlc::set_handler(move || {
                warn_log!("[mac-fan-ctrl] Signal received — restoring fans to Auto");
                restore_fans(&signal_handle);
            })
            .unwrap_or_else(|e| {
                warn_log!("[mac-fan-ctrl] Signal handler registration failed: {e}");
            });

            start_sensor_stream(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::ping_backend,
            commands::get_app_info,
            commands::get_sensors,
            commands::set_fan_constant_rpm,
            commands::set_fan_sensor_control,
            commands::set_fan_auto,
            commands::get_fan_control_configs,
            commands::get_presets,
            commands::get_active_preset,
            commands::apply_preset,
            commands::save_preset,
            commands::delete_preset,
            commands::get_privilege_status,
            commands::request_privilege_restart,
            commands::diagnose_fan_control,
            commands::open_url,
            commands::get_alert_config,
            commands::set_alert_config,
            commands::get_power_preset_config,
            commands::set_power_preset_config,
            commands::get_current_power_source,
        ])
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                // Hide to tray instead of closing
                api.prevent_close();
                let _ = window.hide();
            }
            tauri::WindowEvent::Destroyed => {
                restore_fans(window.app_handle());
            }
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } = &event {
                restore_fans(app_handle);
            }
        });
}
