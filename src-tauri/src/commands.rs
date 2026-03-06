use std::collections::HashMap;
use std::sync::Mutex;

use serde::Serialize;
use tauri::State;

use crate::fan_control::{FanControlConfig, FanControlState};
use crate::presets::{self, Preset, PresetStore};
use crate::smc::SensorService;
use crate::smc_writer::SmcWriter;

pub const SENSOR_UPDATE_EVENT: &str = "sensor_update";

// ── App state shared via Tauri ───────────────────────────────────────────────

pub struct AppState {
    pub fan_control: Mutex<FanControlState>,
    pub smc_writer: Mutex<Option<SmcWriter>>,
    pub preset_store: Mutex<PresetStore>,
}

impl AppState {
    pub fn new() -> Self {
        let writer = SmcWriter::new()
            .map_err(|e| {
                eprintln!("[mac-fan-ctrl] SMC writer init failed (fan control disabled): {e}");
                e
            })
            .ok();

        Self {
            fan_control: Mutex::new(FanControlState::new()),
            smc_writer: Mutex::new(writer),
            preset_store: Mutex::new(presets::load_preset_store()),
        }
    }
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
pub fn get_sensors() -> Result<crate::smc::SensorData, String> {
    let mut service = SensorService::new();
    service.read_all_sensors().map_err(|e| e.to_string())
}

// ── Fan control commands ─────────────────────────────────────────────────────

#[tauri::command]
pub fn set_fan_constant_rpm(
    state: State<'_, AppState>,
    fan_index: u8,
    rpm: f32,
) -> Result<(), String> {
    eprintln!("[cmd] set_fan_constant_rpm: fan_index={fan_index} rpm={rpm}");
    let writer_guard = state.smc_writer.lock().map_err(|e| e.to_string())?;
    let writer = writer_guard
        .as_ref()
        .ok_or_else(|| "SMC writer not available — fan control requires root".to_string())?;

    let mut service = SensorService::new();
    let sensor_data = service.read_all_sensors().map_err(|e| e.to_string())?;

    eprintln!(
        "[cmd] fan data: {:?}",
        sensor_data
            .fans
            .iter()
            .map(|f| format!("fan{}:min={} max={}", f.index, f.min, f.max))
            .collect::<Vec<_>>()
    );

    let config = FanControlConfig::ConstantRpm { target_rpm: rpm };
    let result = state
        .fan_control
        .lock()
        .map_err(|e| e.to_string())?
        .set_config(fan_index, config, &sensor_data.fans, writer)
        .map_err(|e| e.to_string());

    eprintln!("[cmd] set_fan_constant_rpm result: {result:?}");
    result
}

#[tauri::command]
pub fn set_fan_sensor_control(
    state: State<'_, AppState>,
    fan_index: u8,
    sensor_key: String,
    temp_low: f32,
    temp_high: f32,
) -> Result<(), String> {
    let writer_guard = state.smc_writer.lock().map_err(|e| e.to_string())?;
    let writer = writer_guard
        .as_ref()
        .ok_or_else(|| "SMC writer not available — fan control requires root".to_string())?;

    let mut service = SensorService::new();
    let sensor_data = service.read_all_sensors().map_err(|e| e.to_string())?;

    let config = FanControlConfig::SensorBased {
        sensor_key,
        temp_low,
        temp_high,
    };
    state
        .fan_control
        .lock()
        .map_err(|e| e.to_string())?
        .set_config(fan_index, config, &sensor_data.fans, writer)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_fan_auto(state: State<'_, AppState>, fan_index: u8) -> Result<(), String> {
    let writer_guard = state.smc_writer.lock().map_err(|e| e.to_string())?;
    let writer = writer_guard
        .as_ref()
        .ok_or_else(|| "SMC writer not available — fan control requires root".to_string())?;

    state
        .fan_control
        .lock()
        .map_err(|e| e.to_string())?
        .set_auto(fan_index, writer)
        .map_err(|e| e.to_string())
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

    // Get current fan info for Full Blast preset
    let mut service = SensorService::new();
    let sensor_data = service.read_all_sensors().map_err(|e| e.to_string())?;

    let fan_indices: Vec<u8> = sensor_data.fans.iter().map(|f| f.index).collect();
    let fan_maxes: HashMap<u8, f32> = sensor_data.fans.iter().map(|f| (f.index, f.max)).collect();

    Ok(presets::all_presets(&store, &fan_indices, &fan_maxes))
}

#[tauri::command]
pub fn get_active_preset(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let store = state.preset_store.lock().map_err(|e| e.to_string())?;
    Ok(store.active_preset.clone())
}

#[tauri::command]
pub fn apply_preset(state: State<'_, AppState>, name: String) -> Result<(), String> {
    let writer_guard = state.smc_writer.lock().map_err(|e| e.to_string())?;
    let writer = writer_guard
        .as_ref()
        .ok_or_else(|| "SMC writer not available — fan control requires root".to_string())?;

    let mut service = SensorService::new();
    let sensor_data = service.read_all_sensors().map_err(|e| e.to_string())?;

    let fan_indices: Vec<u8> = sensor_data.fans.iter().map(|f| f.index).collect();
    let fan_maxes: HashMap<u8, f32> = sensor_data.fans.iter().map(|f| (f.index, f.max)).collect();

    let mut store = state.preset_store.lock().map_err(|e| e.to_string())?;
    let all = presets::all_presets(&store, &fan_indices, &fan_maxes);

    let preset = all
        .iter()
        .find(|p| p.name == name)
        .ok_or_else(|| format!("Preset '{}' not found", name))?;

    let mut fan_control = state.fan_control.lock().map_err(|e| e.to_string())?;

    // First restore all fans to auto
    fan_control.restore_all_auto(writer);

    // Then apply preset configs
    for (fan_index, config) in &preset.configs {
        fan_control
            .set_config(*fan_index, config.clone(), &sensor_data.fans, writer)
            .map_err(|e| e.to_string())?;
    }

    store.active_preset = Some(name);
    presets::save_preset_store(&store).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn save_preset(state: State<'_, AppState>, name: String) -> Result<(), String> {
    let fan_control = state.fan_control.lock().map_err(|e| e.to_string())?;
    let configs = fan_control.configs().clone();

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

// ── Diagnostic commands ──────────────────────────────────────────────────

#[tauri::command]
pub fn diagnose_fan_control(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    eprintln!("[cmd] diagnose_fan_control called");

    // System info
    let mut lines: Vec<String> = Vec::new();

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
    match writer_guard.as_ref() {
        Some(writer) => {
            lines.push("SMC Writer: AVAILABLE".to_string());
            let diag = writer.diagnose_fan_control();
            lines.extend(diag);
        }
        None => {
            lines.push("SMC Writer: NOT AVAILABLE (init failed — likely not running as root)".to_string());
        }
    }

    // Print to stderr too for terminal visibility
    eprintln!("\n========================================");
    for line in &lines {
        eprintln!("[diag] {line}");
    }
    eprintln!("========================================\n");

    Ok(lines)
}

// ── Privilege commands ───────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct PrivilegeStatus {
    pub has_write_access: bool,
}

#[tauri::command]
pub fn get_privilege_status(state: State<'_, AppState>) -> Result<PrivilegeStatus, String> {
    let writer_guard = state.smc_writer.lock().map_err(|e| e.to_string())?;
    Ok(PrivilegeStatus {
        has_write_access: writer_guard.is_some(),
    })
}

#[tauri::command]
pub fn request_privilege_restart(app_handle: tauri::AppHandle) -> Result<(), String> {
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
    let shell_cmd = format!("'{}' &>/dev/null &", inner_binary.to_string_lossy());

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

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::ping_backend;

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
}
