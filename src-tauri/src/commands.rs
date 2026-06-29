use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::alerts::{self, AlertConfig};
use crate::fan_control::{FanControlConfig, FanControlState};
use crate::fan_snapshot;
use crate::helper_binary;
use crate::log::{debug_log, warn_log};
use crate::power_monitor::PowerSource;
use crate::power_presets::{self, PowerPresetConfig};
use crate::presets::{self, Preset, PresetStore};
use crate::smc::SensorService;
use crate::smc_client::SmcSocketClient;
use crate::smc_writer::{SmcWriteApi, SmcWriter};

pub const SENSOR_UPDATE_EVENT: &str = "sensor_update";

const HELPER_INSTALL_DIR: &str = "/Library/PrivilegedHelperTools";
const LAUNCHDAEMON_DIR: &str = "/Library/LaunchDaemons";
const DAEMON_LABEL: &str = "io.github.naufaldi.fanguard.helper";
const HELPER_READY_RETRY_COUNT: u8 = 20;
const HELPER_READY_RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(250);

// ── Tray handle wrapper ─────────────────────────────────────────────────────

pub struct TrayHandle(pub tauri::tray::TrayIcon);

// ── App state shared via Tauri ───────────────────────────────────────────────

pub struct AppState {
    pub fan_control: Mutex<FanControlState>,
    pub smc_writer: Mutex<Option<Box<dyn SmcWriteApi>>>,
    pub preset_store: Mutex<PresetStore>,
    pub alert_config: Mutex<AlertConfig>,
    pub power_preset_config: Mutex<PowerPresetConfig>,
    pub current_power_source: Mutex<PowerSource>,
}

fn should_try_direct_smc_writer(euid: u32) -> bool {
    euid == 0
}

pub fn distribution_allows_fan_control() -> bool {
    !cfg!(feature = "app-store")
}

fn create_smc_writer() -> Option<Box<dyn SmcWriteApi>> {
    if !distribution_allows_fan_control() {
        debug_log!("[fanguard] Fan control writer disabled for App Store/TestFlight build");
        return None;
    }

    let running_as_root = should_try_direct_smc_writer(unsafe { libc::geteuid() });

    if running_as_root {
        match SmcWriter::new() {
            Ok(writer) => {
                debug_log!("[fanguard] Using direct SMC writer (running as root)");
                return Some(Box::new(writer));
            }
            Err(error) => {
                warn_log!("[fanguard] Direct SMC writer failed while running as root: {error}");
            }
        }
    }

    match SmcSocketClient::new() {
        Ok(client) => {
            debug_log!("[fanguard] Using privileged helper via socket");
            return Some(Box::new(client));
        }
        Err(error) => {
            debug_log!("[fanguard] Privileged helper not available: {error}");
        }
    }

    if running_as_root {
        return None;
    }

    // Unprivileged direct connections can open AppleSMC for reads but fan
    // control keys (F*nMd) return "Unknown key" — do not use as a fallback.
    warn_log!("[fanguard] Fan control disabled — install the privileged helper for write access");
    None
}

impl AppState {
    pub fn new() -> Self {
        let writer = create_smc_writer();

        Self {
            fan_control: Mutex::new(FanControlState::new()),
            smc_writer: Mutex::new(writer),
            preset_store: Mutex::new(presets::load_preset_store()),
            alert_config: Mutex::new(alerts::load_alert_config()),
            power_preset_config: Mutex::new(power_presets::load_power_preset_config()),
            current_power_source: Mutex::new(crate::power_monitor::current_power_source()),
        }
    }
}

// ── App info ────────────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub identifier: String,
}

#[tauri::command]
pub fn get_app_info(app_handle: tauri::AppHandle) -> Result<AppInfo, String> {
    let config = app_handle.config();
    Ok(AppInfo {
        name: config
            .product_name
            .clone()
            .unwrap_or_else(|| "FanGuard".to_string()),
        version: config
            .version
            .clone()
            .unwrap_or_else(|| "0.0.0".to_string()),
        identifier: config.identifier.clone(),
    })
}

// ── Window management ────────────────────────────────────────────────────────

#[tauri::command]
pub fn hide_to_menu_bar(app_handle: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.hide();
    }
    #[cfg(target_os = "macos")]
    let _ = app_handle.set_activation_policy(tauri::ActivationPolicy::Accessory);
    Ok(())
}

// ── Existing commands ────────────────────────────────────────────────────────

#[tauri::command]
pub fn ping_backend(message: String) -> Result<String, String> {
    if message.trim().is_empty() {
        return Err("message must not be empty".to_string());
    }

    Ok(format!("Hello from Rust: {message}"))
}

#[tauri::command]
pub fn get_sensors(app_handle: AppHandle) -> Result<crate::smc::SensorData, String> {
    let mut service = SensorService::new();
    let mut sensor_data = service.read_all_sensors().map_err(|e| e.to_string())?;
    let state = app_handle.state::<AppState>();
    fan_snapshot::overlay_fan_control_on_sensors(&state, &mut sensor_data);
    Ok(sensor_data)
}

fn finish_fan_control_change(app_handle: &AppHandle) -> Result<(), String> {
    let state = app_handle.state::<AppState>();
    let mut service = SensorService::new();
    let sensor_data = service.read_all_sensors().map_err(|e| e.to_string())?;
    let fan_indices: Vec<u8> = sensor_data.fans.iter().map(|f| f.index).collect();
    fan_snapshot::sync_active_preset(&state, &fan_indices)?;
    fan_snapshot::emit_fan_control_snapshot(app_handle);
    Ok(())
}

fn require_writer(
    state: &AppState,
) -> Result<std::sync::MutexGuard<'_, Option<Box<dyn SmcWriteApi>>>, String> {
    if !distribution_allows_fan_control() {
        return Err("Fan control is disabled in TestFlight builds.".to_string());
    }

    state.smc_writer.lock().map_err(|e| e.to_string())
}

fn apply_fan_control_write(
    state: &AppState,
    apply: impl FnOnce(&mut FanControlState, &dyn SmcWriteApi) -> Result<(), String>,
) -> Result<(), String> {
    let writer_guard = require_writer(state)?;
    let writer = writer_guard.as_deref().ok_or_else(|| {
        "Fan control unavailable — grant access to install the privileged helper".to_string()
    })?;
    let mut fan_control = state.fan_control.lock().map_err(|e| e.to_string())?;

    apply(&mut fan_control, writer)
}

pub fn apply_set_fan_constant_rpm(
    app_handle: &AppHandle,
    fan_index: u8,
    rpm: f32,
) -> Result<(), String> {
    if fan_index >= 10 {
        return Err(format!("Invalid fan_index: {fan_index} — must be 0–9"));
    }
    if !rpm.is_finite() || rpm <= 0.0 {
        return Err(format!(
            "Invalid rpm: {rpm} — must be a positive finite number"
        ));
    }

    let state = app_handle.state::<AppState>();
    let mut service = SensorService::new();
    let sensor_data = service.read_all_sensors().map_err(|e| e.to_string())?;
    let config = FanControlConfig::ConstantRpm { target_rpm: rpm };
    apply_fan_control_write(&state, |control, writer| {
        control
            .set_config(fan_index, config, &sensor_data.fans, writer)
            .map_err(|e| e.to_string())
    })?;

    finish_fan_control_change(app_handle)
}

pub fn apply_set_fan_auto(app_handle: &AppHandle, fan_index: u8) -> Result<(), String> {
    if fan_index >= 10 {
        return Err(format!("Invalid fan_index: {fan_index} — must be 0–9"));
    }

    let state = app_handle.state::<AppState>();
    apply_fan_control_write(&state, |control, writer| {
        control
            .set_auto(fan_index, writer)
            .map_err(|e| e.to_string())
    })?;

    finish_fan_control_change(app_handle)
}

// ── Fan control commands ─────────────────────────────────────────────────────

#[tauri::command]
pub fn set_fan_constant_rpm(app_handle: AppHandle, fan_index: u8, rpm: f32) -> Result<(), String> {
    debug_log!("[cmd] set_fan_constant_rpm: fan_index={fan_index} rpm={rpm}");
    apply_set_fan_constant_rpm(&app_handle, fan_index, rpm)
}

#[tauri::command]
pub fn set_fan_sensor_control(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    fan_index: u8,
    sensor_key: String,
    temp_low: f32,
    temp_high: f32,
) -> Result<(), String> {
    if fan_index >= 10 {
        return Err(format!("Invalid fan_index: {fan_index} — must be 0–9"));
    }
    if !temp_low.is_finite() || !temp_high.is_finite() {
        return Err("temp_low and temp_high must be finite numbers".to_string());
    }
    if temp_low >= temp_high {
        return Err(format!(
            "temp_low ({temp_low}) must be strictly less than temp_high ({temp_high})"
        ));
    }

    let mut service = SensorService::new();
    let sensor_data = service.read_all_sensors().map_err(|e| e.to_string())?;

    let sensor_exists = sensor_data.details.iter().any(|s| s.key == sensor_key);
    if !sensor_exists {
        return Err(format!(
            "Sensor key '{sensor_key}' not found in current sensor list"
        ));
    }

    let config = FanControlConfig::SensorBased {
        sensor_key,
        temp_low,
        temp_high,
    };
    apply_fan_control_write(&state, |control, writer| {
        control
            .set_config(fan_index, config, &sensor_data.fans, writer)
            .map_err(|e| e.to_string())
    })?;

    finish_fan_control_change(&app_handle)
}

#[tauri::command]
pub fn set_fan_auto(app_handle: AppHandle, fan_index: u8) -> Result<(), String> {
    apply_set_fan_auto(&app_handle, fan_index)
}

#[tauri::command]
pub fn get_fan_control_configs(
    state: State<'_, AppState>,
) -> Result<HashMap<u8, FanControlConfig>, String> {
    let guard = state.fan_control.lock().map_err(|e| e.to_string())?;
    Ok(guard.configs().clone())
}

// ── Preset commands ──────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_presets(state: State<'_, AppState>) -> Result<Vec<Preset>, String> {
    let store = state.preset_store.lock().map_err(|e| e.to_string())?;

    // Only need fan metadata (indices + max RPM) — skip expensive temperature/ioreg reads
    let mut service = SensorService::new();
    let fans = service.read_fans_only().map_err(|e| e.to_string())?;

    let fan_indices: Vec<u8> = fans.iter().map(|f| f.index).collect();
    let fan_maxes: HashMap<u8, f32> = fans.iter().map(|f| (f.index, f.max)).collect();

    Ok(presets::all_presets(&store, &fan_indices, &fan_maxes))
}

#[tauri::command]
pub fn get_active_preset(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let store = state.preset_store.lock().map_err(|e| e.to_string())?;
    Ok(store.active_preset.clone())
}

#[tauri::command]
pub fn apply_preset(state: State<'_, AppState>, name: String) -> Result<(), String> {
    if !distribution_allows_fan_control() {
        return Err(
            "Fan presets that change fan speed are disabled in TestFlight builds.".to_string(),
        );
    }

    let writer_guard = state.smc_writer.lock().map_err(|e| e.to_string())?;
    let writer = writer_guard.as_deref().ok_or_else(|| {
        "Fan control unavailable — grant access to install the privileged helper".to_string()
    })?;

    // Only need fan metadata — skip expensive temperature/ioreg reads
    let mut service = SensorService::new();
    let fans = service.read_fans_only().map_err(|e| e.to_string())?;

    if fans.is_empty() {
        return Err("No fans detected — cannot apply preset".to_string());
    }

    let fan_indices: Vec<u8> = fans.iter().map(|f| f.index).collect();
    let fan_maxes: HashMap<u8, f32> = fans.iter().map(|f| (f.index, f.max)).collect();

    let mut store = state.preset_store.lock().map_err(|e| e.to_string())?;
    let all = presets::all_presets(&store, &fan_indices, &fan_maxes);

    let preset = all
        .iter()
        .find(|p| p.name == name)
        .ok_or_else(|| format!("Preset '{}' not found", name))?;

    let mut fan_control = state.fan_control.lock().map_err(|e| e.to_string())?;

    // Only restore fans to auto if there are active overrides (avoids unnecessary Ftst toggle)
    if !fan_control.configs().is_empty() {
        fan_control.restore_all_auto(writer);
    }

    // Then apply preset configs
    for (fan_index, config) in &preset.configs {
        fan_control
            .set_config(*fan_index, config.clone(), &fans, writer)
            .map_err(|e| e.to_string())?;
    }

    store.active_preset = Some(name);
    presets::save_preset_store(&store).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn save_preset(state: State<'_, AppState>, name: String) -> Result<(), String> {
    if !distribution_allows_fan_control() {
        return Err("Fan presets are disabled in TestFlight builds.".to_string());
    }

    let fan_control = state.fan_control.lock().map_err(|e| e.to_string())?;
    let configs = fan_control.configs().clone();

    // Only need fan metadata for duplicate detection — skip expensive temperature/ioreg reads
    let mut service = SensorService::new();
    let fans = service.read_fans_only().map_err(|e| e.to_string())?;
    let fan_indices: Vec<u8> = fans.iter().map(|f| f.index).collect();
    let fan_maxes: HashMap<u8, f32> = fans.iter().map(|f| (f.index, f.max)).collect();

    let store = state.preset_store.lock().map_err(|e| e.to_string())?;
    if let Some(existing_name) =
        presets::find_preset_with_matching_configs(&store, &configs, &fan_indices, &fan_maxes)
    {
        return Err(format!("duplicate:{existing_name}"));
    }
    drop(store);

    let preset = Preset {
        name: name.clone(),
        builtin: false,
        configs,
    };

    let mut store = state.preset_store.lock().map_err(|e| e.to_string())?;
    presets::save_custom_preset(&mut store, preset)?;
    store.active_preset = Some(name);
    Ok(())
}

#[tauri::command]
pub fn delete_preset(state: State<'_, AppState>, name: String) -> Result<(), String> {
    let mut store = state.preset_store.lock().map_err(|e| e.to_string())?;
    presets::delete_custom_preset(&mut store, &name)
}

// ── Tray display commands ────────────────────────────────────────────────

#[tauri::command]
pub fn set_tray_display_mode(mode: u8) -> Result<(), String> {
    if mode > 1 {
        return Err(format!(
            "Invalid tray display mode: {mode} — must be 0 (temperature) or 1 (fan RPM)"
        ));
    }
    crate::tray::set_tray_display_mode(mode);
    Ok(())
}

#[tauri::command]
pub fn get_tray_display_mode() -> u8 {
    crate::tray::get_tray_display_mode()
}

// ── Diagnostic commands ──────────────────────────────────────────────────

#[tauri::command]
pub fn diagnose_fan_control(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    debug_log!("[cmd] diagnose_fan_control called");

    // System info
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!(
        "Fan control enabled for this distribution: {}",
        distribution_allows_fan_control()
    ));

    // Check if running as root
    let uid = unsafe { libc::getuid() };
    let euid = unsafe { libc::geteuid() };
    lines.push(format!("Process UID: {uid} EUID: {euid} (0 = root)"));
    lines.push(format!("Running as root: {}", euid == 0));

    // macOS version
    if let Ok(output) = std::process::Command::new("sw_vers").output() {
        let sw_vers = String::from_utf8_lossy(&output.stdout);
        for line in sw_vers.lines() {
            lines.push(format!("  {line}"));
        }
    }

    // Hardware model
    if let Ok(output) = std::process::Command::new("sysctl")
        .arg("-n")
        .arg("hw.model")
        .output()
    {
        let model = String::from_utf8_lossy(&output.stdout).trim().to_string();
        lines.push(format!("Hardware model: {model}"));
    }

    // Chip info
    if let Ok(output) = std::process::Command::new("sysctl")
        .arg("-n")
        .arg("machdep.cpu.brand_string")
        .output()
    {
        let chip = String::from_utf8_lossy(&output.stdout).trim().to_string();
        lines.push(format!("CPU: {chip}"));
    }

    // SMC writer status
    let writer_guard = state.smc_writer.lock().map_err(|e| e.to_string())?;
    match writer_guard.as_deref() {
        Some(writer) => {
            lines.push("SMC Writer: AVAILABLE".to_string());
            let diag = writer.diagnose_fan_control();
            lines.extend(diag);
        }
        None => {
            lines.push(
                "SMC Writer: NOT AVAILABLE (init failed — likely not running as root)".to_string(),
            );
        }
    }

    // Print to stderr too for terminal visibility
    debug_log!("\n========================================");
    for line in &lines {
        debug_log!("[diag] {line}");
    }
    debug_log!("========================================\n");

    Ok(lines)
}

// ── Privilege commands ───────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct PrivilegeStatus {
    pub has_write_access: bool,
    pub fan_control_available: bool,
    pub reason: Option<String>,
}

#[tauri::command]
pub fn get_privilege_status(state: State<'_, AppState>) -> Result<PrivilegeStatus, String> {
    if !distribution_allows_fan_control() {
        return Ok(PrivilegeStatus {
            has_write_access: false,
            fan_control_available: false,
            reason: Some("Fan control is disabled in TestFlight builds.".to_string()),
        });
    }

    let writer_guard = state.smc_writer.lock().map_err(|e| e.to_string())?;
    Ok(PrivilegeStatus {
        has_write_access: writer_guard.is_some(),
        fan_control_available: true,
        reason: None,
    })
}

#[tauri::command]
pub fn request_privilege_restart(app_handle: tauri::AppHandle) -> Result<(), String> {
    if !distribution_allows_fan_control() {
        return Err("Admin restart is disabled in TestFlight builds.".to_string());
    }

    let exe_path =
        std::env::current_exe().map_err(|e| format!("Failed to get executable path: {e}"))?;

    // Look for the .app bundle by traversing up from the binary
    // Binary is at: MyApp.app/Contents/MacOS/my-app
    let app_bundle = exe_path
        .parent() // MacOS/
        .and_then(|p| p.parent()) // Contents/
        .and_then(|p| p.parent()) // MyApp.app/
        .filter(|p| p.extension().is_some_and(|ext| ext == "app"))
        .map(|p| p.to_path_buf());

    // Dev mode (no .app bundle): cannot restart because the Vite dev server
    // is tied to the parent `pnpm tauri dev` process — killing the binary
    // kills Vite too, leaving the new instance with a blank page.
    let app_bundle = app_bundle.ok_or_else(|| {
        "Cannot escalate privileges in development mode. Quit and restart with: sudo pnpm tauri dev".to_string()
    })?;

    // Production: run the inner binary directly as root.
    // NOTE: `open -n` does NOT propagate root privileges (Launch Services
    // always launches as the GUI user), so we must exec the binary directly.
    let inner_binary = app_bundle
        .join("Contents/MacOS")
        .join(exe_path.file_name().unwrap_or_default());
    let shell_cmd = format!(
        "{} &>/dev/null &",
        shell_quote(&inner_binary.to_string_lossy())
    );

    let script = format!("do shell script \"{shell_cmd}\" with administrator privileges");

    let result = std::process::Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|e| format!("Failed to launch osascript: {e}"))?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        // User cancelled the auth dialog — not an error
        if stderr.contains("User canceled") || stderr.contains("-128") {
            return Err("User cancelled the authorization request".to_string());
        }
        return Err(format!("osascript failed: {stderr}"));
    }

    // Exit current unprivileged instance
    app_handle.exit(0);
    Ok(())
}

// ── URL commands ─────────────────────────────────────────────────────────────

#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    const ALLOWED_URLS: [&str; 1] = ["https://github.com/naufaldi/mac-fan-ctrl"];
    if !ALLOWED_URLS.contains(&url.as_str()) {
        return Err("URL is not allowed".to_string());
    }
    std::process::Command::new("/usr/bin/open")
        .arg(&url)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

// ── Alert commands ───────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_alert_config(state: State<'_, AppState>) -> Result<AlertConfig, String> {
    let config = state.alert_config.lock().map_err(|e| e.to_string())?;
    Ok(config.clone())
}

#[derive(Deserialize)]
pub struct SetAlertConfigParams {
    pub enabled: Option<bool>,
    pub cpu_threshold: Option<f64>,
    pub cooldown_secs: Option<u64>,
}

#[tauri::command]
pub fn set_alert_config(
    state: State<'_, AppState>,
    params: SetAlertConfigParams,
) -> Result<AlertConfig, String> {
    let mut config = state.alert_config.lock().map_err(|e| e.to_string())?;

    if let Some(enabled) = params.enabled {
        config.enabled = enabled;
    }
    if let Some(threshold) = params.cpu_threshold {
        if !threshold.is_finite() || !(0.0..=150.0).contains(&threshold) {
            return Err("CPU threshold must be between 0 and 150°C".to_string());
        }
        config.cpu_threshold = threshold;
    }
    if let Some(cooldown) = params.cooldown_secs {
        config.cooldown_secs = cooldown;
    }

    alerts::save_alert_config(&config)?;
    Ok(config.clone())
}

// ── Power preset commands ────────────────────────────────────────────────────

#[tauri::command]
pub fn get_power_preset_config(state: State<'_, AppState>) -> Result<PowerPresetConfig, String> {
    let config = state
        .power_preset_config
        .lock()
        .map_err(|e| e.to_string())?;
    Ok(config.clone())
}

#[derive(Deserialize)]
pub struct SetPowerPresetConfigParams {
    pub enabled: Option<bool>,
    pub ac_preset: Option<Option<String>>,
    pub battery_preset: Option<Option<String>>,
}

#[tauri::command]
pub fn set_power_preset_config(
    state: State<'_, AppState>,
    params: SetPowerPresetConfigParams,
) -> Result<PowerPresetConfig, String> {
    let mut config = state
        .power_preset_config
        .lock()
        .map_err(|e| e.to_string())?;

    if let Some(enabled) = params.enabled {
        config.enabled = enabled;
    }
    if let Some(ac_preset) = params.ac_preset {
        config.ac_preset = ac_preset;
    }
    if let Some(battery_preset) = params.battery_preset {
        config.battery_preset = battery_preset;
    }

    power_presets::save_power_preset_config(&config)?;
    Ok(config.clone())
}

#[tauri::command]
pub fn get_current_power_source(state: State<'_, AppState>) -> Result<PowerSource, String> {
    let source = state
        .current_power_source
        .lock()
        .map_err(|e| e.to_string())?;
    Ok(*source)
}

// ── Helper installation commands ─────────────────────────────────────────────

#[tauri::command]
pub fn install_helper(app_handle: AppHandle) -> Result<String, String> {
    if !distribution_allows_fan_control() {
        return Err("Privileged helper installation is disabled in TestFlight builds.".to_string());
    }

    let exe_path = std::env::current_exe().map_err(|e| format!("Failed to get exe path: {e}"))?;

    let helper_binary = helper_binary::find_helper_binary(&exe_path)?;

    let install_path = format!("{HELPER_INSTALL_DIR}/{DAEMON_LABEL}");
    let plist_path = format!("{LAUNCHDAEMON_DIR}/{DAEMON_LABEL}.plist");

    let plist_content = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{DAEMON_LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{install_path}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardErrorPath</key>
    <string>/tmp/{DAEMON_LABEL}.log</string>
</dict>
</plist>"#
    );

    let shell_commands = build_helper_install_shell_commands(
        &helper_binary,
        &install_path,
        &plist_path,
        &plist_content,
    );

    let script = format!(
        "do shell script {:?} with administrator privileges",
        shell_commands
    );

    let result = std::process::Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|e| format!("Failed to launch osascript: {e}"))?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        if stderr.contains("User canceled") || stderr.contains("-128") {
            return Err("User cancelled the authorization request".to_string());
        }
        return Err(format!("Installation failed: {stderr}"));
    }

    wait_for_helper_ready()?;
    reconnect_writer_internal(&app_handle)?;
    Ok("Helper installed and running".to_string())
}

fn wait_for_helper_ready() -> Result<(), String> {
    wait_for_helper_ready_with(
        HELPER_READY_RETRY_COUNT,
        || {
            SmcSocketClient::new()
                .map(|_| ())
                .map_err(|e| e.to_string())
        },
        || std::thread::sleep(HELPER_READY_RETRY_DELAY),
    )
}

fn wait_for_helper_ready_with<FConnect, FSleep>(
    attempts: u8,
    mut connect: FConnect,
    mut sleep: FSleep,
) -> Result<(), String>
where
    FConnect: FnMut() -> Result<(), String>,
    FSleep: FnMut(),
{
    let mut last_error = "helper did not respond".to_string();
    for attempt in 0..attempts {
        match connect() {
            Ok(()) => return Ok(()),
            Err(error) => {
                last_error = error;
                if attempt + 1 < attempts {
                    sleep();
                }
            }
        }
    }

    Err(format!(
        "Helper installed but socket was not reachable after 5 seconds: {last_error}"
    ))
}

fn build_helper_install_shell_commands(
    helper_binary: &std::path::Path,
    install_path: &str,
    plist_path: &str,
    plist_content: &str,
) -> String {
    let helper_dir = shell_quote(HELPER_INSTALL_DIR);
    let helper_binary = shell_quote(&helper_binary.to_string_lossy());
    let install_path = shell_quote(install_path);
    let plist_path = shell_quote(plist_path);

    format!(
        "mkdir -p {helper_dir} && cp {helper_binary} {install_path} && chmod 755 {install_path} && chown root:wheel {install_path} && /bin/cat > {plist_path} << 'PLISTEOF'\n{plist_content}\nPLISTEOF\nchown root:wheel {plist_path} && chmod 644 {plist_path} && launchctl bootout system {plist_path} 2>/dev/null || true\nlaunchctl enable system/{label}\nlaunchctl bootstrap system {plist_path}\nlaunchctl kickstart -k system/{label}",
        helper_dir = helper_dir,
        helper_binary = helper_binary,
        install_path = install_path,
        plist_path = plist_path,
        plist_content = plist_content,
        label = DAEMON_LABEL,
    )
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn reconnect_writer_internal(app_handle: &AppHandle) -> Result<bool, String> {
    let state = app_handle.state::<AppState>();
    let client = SmcSocketClient::new().map_err(|e| e.to_string())?;
    let mut writer_guard = state.smc_writer.lock().map_err(|e| e.to_string())?;
    *writer_guard = Some(Box::new(client));
    Ok(true)
}

#[tauri::command]
pub fn reconnect_writer(app_handle: AppHandle) -> Result<bool, String> {
    if !distribution_allows_fan_control() {
        return Err("Fan control reconnection is disabled in TestFlight builds.".to_string());
    }

    reconnect_writer_internal(&app_handle)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::{
        apply_fan_control_write, build_helper_install_shell_commands,
        distribution_allows_fan_control, ping_backend, should_try_direct_smc_writer,
        wait_for_helper_ready_with, DAEMON_LABEL, HELPER_INSTALL_DIR, LAUNCHDAEMON_DIR,
    };
    use crate::alerts::AlertConfig;
    use crate::fan_control::{FanControlConfig, FanControlState};
    use crate::power_monitor::PowerSource;
    use crate::power_presets::PowerPresetConfig;
    use crate::presets::PresetStore;
    use crate::smc::{FanData, FanMode};
    use crate::smc_writer::mock::MockSmcWriter;
    use std::path::Path;
    use std::sync::Mutex;

    fn make_fan() -> FanData {
        FanData {
            index: 0,
            label: "Fan 0".to_string(),
            actual: 2000.0,
            min: 1200.0,
            max: 6550.0,
            target: 1200.0,
            mode: FanMode::Auto,
        }
    }

    fn test_state() -> super::AppState {
        super::AppState {
            fan_control: Mutex::new(FanControlState::new()),
            smc_writer: Mutex::new(Some(Box::new(MockSmcWriter::new()))),
            preset_store: Mutex::new(PresetStore {
                active_preset: Some("Automatic".to_string()),
                custom_presets: vec![],
            }),
            alert_config: Mutex::new(AlertConfig {
                enabled: false,
                cpu_threshold: 90.0,
                cooldown_secs: 60,
            }),
            power_preset_config: Mutex::new(PowerPresetConfig {
                enabled: false,
                ac_preset: None,
                battery_preset: None,
            }),
            current_power_source: Mutex::new(PowerSource::Unknown),
        }
    }

    #[test]
    fn ping_backend_returns_expected_payload() {
        let result = ping_backend("Hello world".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap_or_default(), "Hello from Rust: Hello world");
    }

    #[test]
    fn ping_backend_rejects_empty_message() {
        let result = ping_backend(String::new());
        assert!(result.is_err());
    }

    #[test]
    fn should_try_direct_smc_writer_when_running_as_root() {
        assert!(should_try_direct_smc_writer(0));
    }

    #[test]
    fn should_not_try_direct_smc_writer_when_running_unprivileged() {
        assert!(!should_try_direct_smc_writer(501));
    }

    #[test]
    fn helper_install_commands_enable_service_before_bootstrap() {
        let helper_binary = Path::new("/tmp/fanguard-helper");
        let install_path = format!("{HELPER_INSTALL_DIR}/{DAEMON_LABEL}");
        let plist_path = format!("{LAUNCHDAEMON_DIR}/{DAEMON_LABEL}.plist");
        let plist_content = "<plist />".to_string();

        let commands = build_helper_install_shell_commands(
            helper_binary,
            &install_path,
            &plist_path,
            &plist_content,
        );

        assert!(commands.contains(&format!("launchctl enable system/{DAEMON_LABEL}")));
    }

    #[test]
    fn helper_install_commands_escape_single_quotes_in_paths() {
        let helper_binary = Path::new("/tmp/fan'guard-helper");
        let install_path = "/Library/PrivilegedHelperTools/io.github.naufaldi.fanguard.helper";
        let plist_path = "/Library/LaunchDaemons/io.github.naufaldi.fanguard.helper.plist";

        let commands = build_helper_install_shell_commands(
            helper_binary,
            install_path,
            plist_path,
            "<plist />",
        );

        assert!(commands.contains("'/tmp/fan'\"'\"'guard-helper'"));
        assert!(!commands.contains("cp '/tmp/fan'guard-helper'"));
    }

    #[test]
    fn helper_ready_wait_requires_successful_socket_ping() {
        let mut attempts = 0u8;
        let result = wait_for_helper_ready_with(
            3,
            || {
                attempts += 1;
                if attempts < 2 {
                    Err("Permission denied".to_string())
                } else {
                    Ok(())
                }
            },
            || {},
        );

        assert!(result.is_ok());
        assert_eq!(attempts, 2);
    }

    #[test]
    fn helper_ready_wait_reports_last_socket_ping_error() {
        let result = wait_for_helper_ready_with(2, || Err("Permission denied".to_string()), || {});

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Permission denied"));
    }

    #[test]
    fn fan_control_write_helper_releases_locks_before_snapshot_refresh() {
        let state = test_state();
        let fans = vec![make_fan()];

        apply_fan_control_write(&state, |control, writer| {
            control
                .set_config(
                    0,
                    FanControlConfig::ConstantRpm { target_rpm: 4000.0 },
                    &fans,
                    writer,
                )
                .map_err(|e| e.to_string())
        })
        .expect("fan control write should succeed");

        assert!(state.smc_writer.try_lock().is_ok());
        assert!(state.fan_control.try_lock().is_ok());
    }

    #[cfg(not(feature = "app-store"))]
    #[test]
    fn direct_distribution_allows_fan_control_by_default() {
        assert!(distribution_allows_fan_control());
    }

    #[cfg(feature = "app-store")]
    #[test]
    fn app_store_distribution_disables_fan_control() {
        assert!(!distribution_allows_fan_control());
    }
}
