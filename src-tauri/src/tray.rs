//! macOS menu bar (system tray) integration.
//!
//! Shows a fan icon + CPU temperature in the menu bar. The dropdown menu
//! provides quick access to fan controls, presets, and the main window.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use tauri::menu::{CheckMenuItem, Menu, MenuBuilder, MenuItem, PredefinedMenuItem, SubmenuBuilder};
use tauri::tray::{TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{image::Image, AppHandle, Emitter, Manager};

/// Timestamp (millis since epoch) of last tray icon click.
/// While the menu is likely open (within MENU_GUARD_MS of a click),
/// we skip `set_menu()` calls to avoid macOS dismissing the dropdown.
static LAST_TRAY_CLICK_MS: AtomicU64 = AtomicU64::new(0);

/// How long (ms) to guard the menu from rebuilds after a click.
/// 15 seconds is generous — users rarely keep a menu open longer.
const MENU_GUARD_MS: u64 = 15_000;

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn is_menu_guarded() -> bool {
    let last_click = LAST_TRAY_CLICK_MS.load(Ordering::Relaxed);
    if last_click == 0 {
        return false;
    }
    let elapsed = now_millis().saturating_sub(last_click);
    elapsed < MENU_GUARD_MS
}

fn mark_menu_opened() {
    LAST_TRAY_CLICK_MS.store(now_millis(), Ordering::Relaxed);
}

fn mark_menu_closed() {
    LAST_TRAY_CLICK_MS.store(0, Ordering::Relaxed);
}

use crate::commands::{AppState, TrayHandle};
use crate::fan_control::FanControlConfig;
use crate::log::{debug_log, warn_log};
use crate::power_monitor::PowerSource;
use crate::presets;
use crate::smc::{FanData, FanMode, SensorData, SensorService};

// ── Menu item ID constants ──────────────────────────────────────────────────

const SHOW_WINDOW: &str = "show_window";
const ABOUT: &str = "about";
const CHECK_FOR_UPDATES: &str = "check_for_updates";
const QUIT: &str = "quit";
const PRESET_PREFIX: &str = "preset::";
const FAN_AUTO_PREFIX: &str = "fan_auto::";
const FAN_RPM_PREFIX: &str = "fan_rpm::";

// ── Setup ───────────────────────────────────────────────────────────────────

pub fn setup_tray(app: &mut tauri::App) -> Result<TrayIcon, tauri::Error> {
    let icon_bytes = include_bytes!("../icons/menu-icon-template@2x.png");
    let icon = Image::from_bytes(icon_bytes)?;

    let initial_menu = build_initial_menu(app.handle())?;

    TrayIconBuilder::new()
        .icon(icon)
        .icon_as_template(true)
        .title("--°C")
        .tooltip("FanGuard Beta")
        .menu(&initial_menu)
        .show_menu_on_left_click(true)
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(handle_tray_icon_event)
        .build(app)
}

fn build_initial_menu(app: &AppHandle) -> Result<Menu<tauri::Wry>, tauri::Error> {
    let show_item = MenuItem::with_id(app, SHOW_WINDOW, "Show FanGuard", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let about_item = MenuItem::with_id(app, ABOUT, "About FanGuard", true, None::<&str>)?;
    let update_item = MenuItem::with_id(app, CHECK_FOR_UPDATES, "Check for Updates...", true, None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, QUIT, "Quit FanGuard", true, None::<&str>)?;

    MenuBuilder::new(app)
        .item(&show_item)
        .item(&sep1)
        .item(&about_item)
        .item(&update_item)
        .item(&sep2)
        .item(&quit_item)
        .build()
}

// ── Menu construction ───────────────────────────────────────────────────────

fn build_tray_menu(
    app: &AppHandle,
    fans: &[FanData],
    active_preset: Option<&str>,
    all_presets: &[presets::Preset],
    power_source: PowerSource,
) -> Result<Menu<tauri::Wry>, tauri::Error> {
    let show_item = MenuItem::with_id(app, SHOW_WINDOW, "Show FanGuard", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;

    // Power source indicator
    let power_label = match power_source {
        PowerSource::Ac => "Power: AC",
        PowerSource::Battery => "Power: Battery",
        PowerSource::Unknown => "Power: Unknown",
    };
    let power_item = MenuItem::with_id(app, "power_source", power_label, false, None::<&str>)?;

    // Available fans section
    let fans_header = MenuItem::with_id(app, "fans_header", "Available fans:", false, None::<&str>)?;

    let fan_submenus: Vec<tauri::menu::Submenu<tauri::Wry>> = fans
        .iter()
        .filter_map(|fan| build_fan_submenu(app, fan).ok())
        .collect();

    let sep2 = PredefinedMenuItem::separator(app)?;

    // Presets section
    let presets_header =
        MenuItem::with_id(app, "presets_header", "Fan presets:", false, None::<&str>)?;

    let preset_items: Vec<CheckMenuItem<tauri::Wry>> = all_presets
        .iter()
        .filter_map(|p| {
            let id = format!("{PRESET_PREFIX}{}", p.name);
            let checked = active_preset == Some(p.name.as_str());
            CheckMenuItem::with_id(app, &id, &p.name, true, checked, None::<&str>).ok()
        })
        .collect();

    let sep3 = PredefinedMenuItem::separator(app)?;
    let about_item = MenuItem::with_id(app, ABOUT, "About FanGuard", true, None::<&str>)?;
    let update_item = MenuItem::with_id(app, CHECK_FOR_UPDATES, "Check for Updates...", true, None::<&str>)?;
    let sep4 = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, QUIT, "Quit FanGuard", true, None::<&str>)?;

    // Assemble the menu
    let mut builder = MenuBuilder::new(app)
        .item(&show_item)
        .item(&sep1)
        .item(&power_item)
        .item(&fans_header);

    for submenu in &fan_submenus {
        builder = builder.item(submenu);
    }

    builder = builder.item(&sep2).item(&presets_header);

    for preset_item in &preset_items {
        builder = builder.item(preset_item);
    }

    builder
        .item(&sep3)
        .item(&about_item)
        .item(&update_item)
        .item(&sep4)
        .item(&quit_item)
        .build()
}

fn build_fan_submenu(
    app: &AppHandle,
    fan: &FanData,
) -> Result<tauri::menu::Submenu<tauri::Wry>, tauri::Error> {
    let mode_label = match fan.mode {
        FanMode::Auto => "Auto".to_string(),
        FanMode::Forced => format!("{} RPM", fan.target as u32),
    };
    let title = format!("{} – {mode_label}", fan.label);

    let auto_id = format!("{FAN_AUTO_PREFIX}{}", fan.index);
    let is_auto = matches!(fan.mode, FanMode::Auto);
    let auto_item = CheckMenuItem::with_id(app, &auto_id, "Auto", true, is_auto, None::<&str>)?;

    // RPM steps: 25%, 50%, 75%, 100% of max
    let rpm_ratios = [0.25_f32, 0.50, 0.75, 1.00];
    let rpm_items: Vec<CheckMenuItem<tauri::Wry>> = rpm_ratios
        .iter()
        .filter_map(|ratio| {
            let rpm = (fan.max * ratio) as u32;
            let id = format!("{FAN_RPM_PREFIX}{}::{rpm}", fan.index);
            let label = format!("{rpm} RPM");
            let checked =
                matches!(fan.mode, FanMode::Forced) && (fan.target as u32).abs_diff(rpm) < 50;
            CheckMenuItem::with_id(app, &id, &label, true, checked, None::<&str>).ok()
        })
        .collect();

    let mut sub = SubmenuBuilder::new(app, &title).item(&auto_item);

    for rpm_item in &rpm_items {
        sub = sub.item(rpm_item);
    }

    sub.build()
}

// ── Tray updates (called from sensor stream) ────────────────────────────────

pub fn update_tray_title(app_handle: &AppHandle, sensor_data: &SensorData) {
    let cpu_temp = sensor_data
        .summary
        .cpu_package
        .as_ref()
        .and_then(|s| s.value);

    let is_alert = app_handle
        .try_state::<crate::commands::AppState>()
        .and_then(|state| state.alert_config.lock().ok().map(|c| c.clone()))
        .is_some_and(|config| {
            config.enabled && cpu_temp.is_some_and(|t| t >= config.cpu_threshold)
        });

    let prefix = if is_alert { "⚠️ " } else { "" };
    let temp_str = cpu_temp
        .map(|v| format!("{prefix}{v:.0}°C"))
        .unwrap_or_else(|| "--°C".to_string());

    if let Some(tray_state) = app_handle.try_state::<TrayHandle>() {
        debug_log!("[tray] set_title({temp_str})");
        let _ = tray_state.0.set_title(Some(&temp_str));
    }
}

pub fn update_tray_menu(app_handle: &AppHandle, sensor_data: &SensorData) {
    // Skip menu rebuild while the dropdown is likely open to avoid dismissing it
    if is_menu_guarded() {
        return;
    }

    let Some(tray_state) = app_handle.try_state::<TrayHandle>() else {
        return;
    };

    let app_state = app_handle.state::<AppState>();

    let active_preset = app_state
        .preset_store
        .lock()
        .ok()
        .and_then(|s| s.active_preset.clone());

    let fan_indices: Vec<u8> = sensor_data.fans.iter().map(|f| f.index).collect();
    let fan_maxes: HashMap<u8, f32> = sensor_data.fans.iter().map(|f| (f.index, f.max)).collect();

    let all_presets = app_state
        .preset_store
        .lock()
        .ok()
        .map(|s| presets::all_presets(&s, &fan_indices, &fan_maxes))
        .unwrap_or_default();

    let power_source = app_state
        .current_power_source
        .lock()
        .ok()
        .map(|g| *g)
        .unwrap_or(PowerSource::Unknown);

    match build_tray_menu(
        app_handle,
        &sensor_data.fans,
        active_preset.as_deref(),
        &all_presets,
        power_source,
    ) {
        Ok(menu) => {
            debug_log!(
                "[tray] set_menu fans={} presets={}",
                sensor_data.fans.len(),
                all_presets.len()
            );
            let _ = tray_state.0.set_menu(Some(menu));
        }
        Err(e) => {
            warn_log!("[tray] build_tray_menu FAILED: {e}");
        }
    }
}

// ── Event handlers ──────────────────────────────────────────────────────────

fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    let id = event.id().as_ref();
    // Menu item was selected → menu is now closed
    mark_menu_closed();
    debug_log!("[tray] menu_event: id={id:?}");

    match id {
        SHOW_WINDOW => show_main_window(app),
        ABOUT => {
            show_main_window(app);
            let _ = app.emit("show-about", ());
        }
        CHECK_FOR_UPDATES => {
            show_main_window(app);
            let _ = app.emit("check-for-updates", ());
        }
        QUIT => quit_app(app),
        _ if id.starts_with(PRESET_PREFIX) => {
            let preset_name = &id[PRESET_PREFIX.len()..];
            apply_preset_from_tray(app, preset_name);
        }
        _ if id.starts_with(FAN_AUTO_PREFIX) => {
            if let Ok(fan_index) = id[FAN_AUTO_PREFIX.len()..].parse::<u8>() {
                set_fan_auto_from_tray(app, fan_index);
            }
        }
        _ if id.starts_with(FAN_RPM_PREFIX) => {
            let rest = &id[FAN_RPM_PREFIX.len()..];
            if let Some((idx_str, rpm_str)) = rest.split_once("::") {
                if let (Ok(fan_index), Ok(rpm)) = (idx_str.parse::<u8>(), rpm_str.parse::<f32>()) {
                    set_fan_rpm_from_tray(app, fan_index, rpm);
                }
            }
        }
        _ => {}
    }
}

fn handle_tray_icon_event(tray: &TrayIcon, event: TrayIconEvent) {
    match &event {
        TrayIconEvent::Click {
            button,
            button_state,
            ..
        } => {
            mark_menu_opened();
            debug_log!("[tray] icon_event: Click button={button:?} state={button_state:?}");
        }
        TrayIconEvent::DoubleClick { .. } => {
            debug_log!("[tray] icon_event: DoubleClick");
            show_main_window(tray.app_handle());
        }
        TrayIconEvent::Enter { .. } => {
            debug_log!("[tray] icon_event: Enter");
        }
        TrayIconEvent::Leave { .. } => {
            debug_log!("[tray] icon_event: Leave");
        }
        TrayIconEvent::Move { .. } => {} // too noisy
        _ => {
            debug_log!("[tray] icon_event: other");
        }
    }
}

// ── Action helpers ──────────────────────────────────────────────────────────

fn show_main_window(app: &AppHandle) {
    // Restore Dock + Cmd+Tab presence before showing
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);

    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.unminimize();
    }
}

fn quit_app(app: &AppHandle) {
    let state = app.state::<AppState>();
    if let (Ok(writer_guard), Ok(mut control)) =
        (state.smc_writer.lock(), state.fan_control.lock())
    {
        if let Some(writer) = writer_guard.as_deref() {
            control.restore_all_auto(writer);
        }
    }
    app.exit(0);
}

fn apply_preset_from_tray(app: &AppHandle, preset_name: &str) {
    let name = preset_name.to_string();
    let app = app.clone();

    std::thread::spawn(move || {
        crate::apply_preset_by_name(&app, &name);
    });
}

fn set_fan_auto_from_tray(app: &AppHandle, fan_index: u8) {
    let app = app.clone();

    std::thread::spawn(move || {
        let state = app.state::<AppState>();
        let writer_guard = match state.smc_writer.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        let Some(writer) = writer_guard.as_deref() else {
            return;
        };
        let mut control = match state.fan_control.lock() {
            Ok(c) => c,
            Err(_) => return,
        };
        let _ = control.set_auto(fan_index, writer);
    });
}

fn set_fan_rpm_from_tray(app: &AppHandle, fan_index: u8, rpm: f32) {
    let app = app.clone();

    std::thread::spawn(move || {
        let state = app.state::<AppState>();
        let writer_guard = match state.smc_writer.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        let Some(writer) = writer_guard.as_deref() else {
            return;
        };

        let mut service = SensorService::new();
        let fans = service.read_fans_only();

        let config = FanControlConfig::ConstantRpm { target_rpm: rpm };
        let mut control = match state.fan_control.lock() {
            Ok(c) => c,
            Err(_) => return,
        };
        let _ = control.set_config(fan_index, config, &fans, writer);
    });
}
