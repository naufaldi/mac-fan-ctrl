//! macOS menu bar (system tray) integration.
//!
//! Shows a fan icon + CPU temperature in the menu bar. The dropdown menu
//! provides quick access to fan controls, presets, and the main window.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};

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

/// Tray display mode: 0 = temperature (default), 1 = fan RPM
static TRAY_DISPLAY_MODE: AtomicU8 = AtomicU8::new(0);

/// Set the tray display mode (0 = temperature, 1 = fan RPM).
pub fn set_tray_display_mode(mode: u8) {
    TRAY_DISPLAY_MODE.store(mode.min(1), Ordering::Relaxed);
}

/// Get the current tray display mode.
pub fn get_tray_display_mode() -> u8 {
    TRAY_DISPLAY_MODE.load(Ordering::Relaxed)
}

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
use crate::log::{debug_log, warn_log};
use crate::power_monitor::PowerSource;
use crate::presets;
use crate::smc::{FanData, FanMode, SensorData};

// ── Menu item ID constants ──────────────────────────────────────────────────

const SHOW_WINDOW: &str = "show_window";
const ABOUT: &str = "about";
const CHECK_FOR_UPDATES: &str = "check_for_updates";
const QUIT: &str = "quit";
const PRESET_PREFIX: &str = "preset::";
const FAN_AUTO_PREFIX: &str = "fan_auto::";
const FAN_RPM_PREFIX: &str = "fan_rpm::";
const RPM_TOLERANCE: u32 = 50;
const RPM_RATIOS: [f32; 4] = [0.25, 0.50, 0.75, 1.00];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FanModeMenuAction {
    Auto,
    Rpm(u32),
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FanModeMenuRow {
    pub id_suffix: String,
    pub label: String,
    pub selected: bool,
    pub enabled: bool,
    pub action: FanModeMenuAction,
}

fn selected_menu_label(selected: bool, text: &str) -> String {
    if selected {
        format!("✓ {text}")
    } else {
        text.to_string()
    }
}

/// Builds mutually exclusive fan mode rows for tray submenu rendering.
#[cfg(test)]
pub fn fan_mode_menu_rows(fan: &FanData) -> Vec<FanModeMenuRow> {
    fan_mode_menu_rows_for_availability(fan, true)
}

/// Builds fan mode rows with write controls removed for monitoring-only builds.
pub fn fan_mode_menu_rows_for_availability(
    fan: &FanData,
    fan_control_available: bool,
) -> Vec<FanModeMenuRow> {
    if !fan_control_available {
        return vec![FanModeMenuRow {
            id_suffix: "monitoring".to_string(),
            label: "Monitoring only".to_string(),
            selected: false,
            enabled: false,
            action: FanModeMenuAction::Unavailable,
        }];
    }

    let preset_rows: Vec<(u32, String)> = RPM_RATIOS
        .iter()
        .map(|ratio| {
            let rpm = (fan.max * ratio) as u32;
            (rpm, format!("{rpm} RPM"))
        })
        .collect();

    match fan.mode {
        FanMode::Auto => {
            let mut rows = vec![FanModeMenuRow {
                id_suffix: "auto".to_string(),
                label: selected_menu_label(true, "Auto"),
                selected: true,
                enabled: true,
                action: FanModeMenuAction::Auto,
            }];
            rows.extend(preset_rows.into_iter().map(|(rpm, label)| FanModeMenuRow {
                id_suffix: rpm.to_string(),
                label: selected_menu_label(false, &label),
                selected: false,
                enabled: true,
                action: FanModeMenuAction::Rpm(rpm),
            }));
            rows
        }
        FanMode::Forced => {
            let target = fan.target as u32;
            let matched_rpm = preset_rows
                .iter()
                .find(|(rpm, _)| rpm.abs_diff(target) < RPM_TOLERANCE)
                .map(|(rpm, _)| *rpm);

            let mut rows = vec![FanModeMenuRow {
                id_suffix: "auto".to_string(),
                label: selected_menu_label(false, "Auto"),
                selected: false,
                enabled: true,
                action: FanModeMenuAction::Auto,
            }];

            rows.extend(preset_rows.into_iter().map(|(rpm, label)| {
                let selected = matched_rpm == Some(rpm);
                FanModeMenuRow {
                    id_suffix: rpm.to_string(),
                    label: selected_menu_label(selected, &label),
                    selected,
                    enabled: true,
                    action: FanModeMenuAction::Rpm(rpm),
                }
            }));

            if matched_rpm.is_none() {
                rows.push(FanModeMenuRow {
                    id_suffix: "custom".to_string(),
                    label: selected_menu_label(true, &format!("Custom ({target} RPM)")),
                    selected: true,
                    enabled: true,
                    action: FanModeMenuAction::Rpm(target),
                });
            }

            rows
        }
    }
}

fn fan_mode_row_id(fan_index: u8, row: &FanModeMenuRow) -> String {
    match &row.action {
        FanModeMenuAction::Auto => format!("{FAN_AUTO_PREFIX}{fan_index}"),
        FanModeMenuAction::Rpm(rpm) => format!("{FAN_RPM_PREFIX}{fan_index}::{rpm}"),
        FanModeMenuAction::Unavailable => format!("fan_unavailable::{fan_index}"),
    }
}

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
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, QUIT, "Quit FanGuard", true, None::<&str>)?;

    let builder = MenuBuilder::new(app)
        .item(&show_item)
        .item(&sep1)
        .item(&about_item);

    let builder = if crate::commands::distribution_allows_fan_control() {
        let update_item = MenuItem::with_id(
            app,
            CHECK_FOR_UPDATES,
            "Check for Updates...",
            true,
            None::<&str>,
        )?;
        builder.item(&update_item)
    } else {
        builder
    };

    builder.item(&sep2).item(&quit_item).build()
}

// ── Menu construction ───────────────────────────────────────────────────────

fn build_tray_menu(
    app: &AppHandle,
    fans: &[FanData],
    active_preset: Option<&str>,
    all_presets: &[presets::Preset],
    power_source: PowerSource,
) -> Result<Menu<tauri::Wry>, tauri::Error> {
    let fan_control_available = crate::commands::distribution_allows_fan_control();
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
    let fans_header =
        MenuItem::with_id(app, "fans_header", "Available fans:", false, None::<&str>)?;

    let fan_submenus: Vec<tauri::menu::Submenu<tauri::Wry>> = fans
        .iter()
        .filter_map(|fan| build_fan_submenu(app, fan, fan_control_available).ok())
        .collect();

    let sep2 = PredefinedMenuItem::separator(app)?;

    // Presets section
    let sep3 = PredefinedMenuItem::separator(app)?;
    let about_item = MenuItem::with_id(app, ABOUT, "About FanGuard", true, None::<&str>)?;
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

    if fan_control_available {
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

        builder = builder.item(&sep2).item(&presets_header);

        for preset_item in &preset_items {
            builder = builder.item(preset_item);
        }
    }

    let builder = builder.item(&sep3).item(&about_item);

    let builder = if fan_control_available {
        let update_item = MenuItem::with_id(
            app,
            CHECK_FOR_UPDATES,
            "Check for Updates...",
            true,
            None::<&str>,
        )?;
        builder.item(&update_item)
    } else {
        builder
    };

    builder.item(&sep4).item(&quit_item).build()
}

fn build_fan_submenu(
    app: &AppHandle,
    fan: &FanData,
    fan_control_available: bool,
) -> Result<tauri::menu::Submenu<tauri::Wry>, tauri::Error> {
    let mode_label = match fan.mode {
        FanMode::Auto => "Auto".to_string(),
        FanMode::Forced => format!("{} RPM", fan.target as u32),
    };
    let title = format!("{} – {mode_label}", fan.label);

    let rows = fan_mode_menu_rows_for_availability(fan, fan_control_available);
    let mut sub = SubmenuBuilder::new(app, &title);

    for row in &rows {
        let item = MenuItem::with_id(
            app,
            &fan_mode_row_id(fan.index, row),
            &row.label,
            row.enabled,
            None::<&str>,
        )?;
        sub = sub.item(&item);
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

    let title = match get_tray_display_mode() {
        1 => {
            // Fan RPM mode: show first fan's actual RPM
            sensor_data
                .fans
                .first()
                .map(|f| format!("{} RPM", f.actual as u32))
                .unwrap_or_else(|| "-- RPM".to_string())
        }
        _ => {
            // Temperature mode (default) with alert prefix
            cpu_temp
                .map(|v| format!("{prefix}{v:.0}°C"))
                .unwrap_or_else(|| "--°C".to_string())
        }
    };

    if let Some(tray_state) = app_handle.try_state::<TrayHandle>() {
        debug_log!("[tray] set_title({title})");
        let _ = tray_state.0.set_title(Some(&title));
    }
}

pub fn update_tray_menu(app_handle: &AppHandle, sensor_data: &SensorData) {
    rebuild_tray_menu(app_handle, sensor_data, false);
}

pub fn update_tray_menu_force(app_handle: &AppHandle, sensor_data: &SensorData) {
    rebuild_tray_menu(app_handle, sensor_data, true);
}

fn rebuild_tray_menu(app_handle: &AppHandle, sensor_data: &SensorData, force: bool) {
    if !force && is_menu_guarded() {
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
                "[tray] set_menu fans={} presets={} force={force}",
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
            if !crate::commands::distribution_allows_fan_control() {
                return;
            }
            show_main_window(app);
            let _ = app.emit("check-for-updates", ());
        }
        QUIT => quit_app(app),
        _ if id.starts_with(PRESET_PREFIX) => {
            if !crate::commands::distribution_allows_fan_control() {
                return;
            }
            let preset_name = &id[PRESET_PREFIX.len()..];
            apply_preset_from_tray(app, preset_name);
        }
        _ if id.starts_with(FAN_AUTO_PREFIX) => {
            if !crate::commands::distribution_allows_fan_control() {
                return;
            }
            if let Ok(fan_index) = id[FAN_AUTO_PREFIX.len()..].parse::<u8>() {
                set_fan_auto_from_tray(app, fan_index);
            }
        }
        _ if id.starts_with(FAN_RPM_PREFIX) => {
            if !crate::commands::distribution_allows_fan_control() {
                return;
            }
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
    crate::show_quit_confirmation(app);
}

fn apply_preset_from_tray(app: &AppHandle, preset_name: &str) {
    let name = preset_name.to_string();
    let app = app.clone();

    std::thread::spawn(move || {
        crate::apply_preset_by_name(&app, &name);
    });
}

fn set_fan_auto_from_tray(app: &AppHandle, fan_index: u8) {
    if let Err(error) = crate::commands::apply_set_fan_auto(app, fan_index) {
        warn_log!("[fanguard] Tray set fan auto failed: {error}");
    }
}

fn set_fan_rpm_from_tray(app: &AppHandle, fan_index: u8, rpm: f32) {
    if let Err(error) = crate::commands::apply_set_fan_constant_rpm(app, fan_index, rpm) {
        warn_log!("[fanguard] Tray set fan RPM failed: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fan(mode: FanMode, target: f32, max: f32) -> FanData {
        FanData {
            index: 0,
            label: "Fan 0".to_string(),
            actual: 2000.0,
            min: 1200.0,
            max,
            target,
            mode,
        }
    }

    #[test]
    fn fan_mode_menu_rows_auto_selects_only_auto() {
        let rows = fan_mode_menu_rows(&make_fan(FanMode::Auto, 1200.0, 6550.0));
        let selected: Vec<_> = rows.iter().filter(|row| row.selected).collect();
        assert_eq!(selected.len(), 1);
        assert!(matches!(selected[0].action, FanModeMenuAction::Auto));
    }

    #[test]
    fn fan_mode_menu_rows_forced_near_step_selects_one_rpm() {
        let max = 6550.0;
        let rpm = (max * 0.50) as u32;
        let rows = fan_mode_menu_rows(&make_fan(FanMode::Forced, rpm as f32, max));
        let selected: Vec<_> = rows.iter().filter(|row| row.selected).collect();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].action, FanModeMenuAction::Rpm(rpm));
    }

    #[test]
    fn fan_mode_menu_rows_forced_custom_selects_custom_label() {
        let rows = fan_mode_menu_rows(&make_fan(FanMode::Forced, 4000.0, 6550.0));
        let selected: Vec<_> = rows.iter().filter(|row| row.selected).collect();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].action, FanModeMenuAction::Rpm(4000));
        assert!(selected[0].label.contains("Custom"));
    }

    #[test]
    fn fan_mode_menu_rows_monitoring_only_do_not_expose_write_actions() {
        let rows =
            fan_mode_menu_rows_for_availability(&make_fan(FanMode::Auto, 1200.0, 6550.0), false);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].label, "Monitoring only");
        assert!(!rows[0].enabled);
        assert_eq!(rows[0].action, FanModeMenuAction::Unavailable);
    }
}
