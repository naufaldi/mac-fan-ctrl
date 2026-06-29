//! Fan control snapshot emission and preset synchronization.

use tauri::{AppHandle, Emitter, Manager};

use crate::commands::{AppState, SENSOR_UPDATE_EVENT};
use crate::fan_control::FanControlConfig;
use crate::log::warn_log;
use crate::presets;
use crate::smc::SensorService;
use crate::tray;

const AUTOMATIC_PRESET: &str = "Automatic";

/// Clears active preset when any fan is manually forced; sets Automatic when all are Auto.
pub fn sync_active_preset(state: &AppState, fan_indices: &[u8]) -> Result<(), String> {
    let configs = state
        .fan_control
        .lock()
        .map_err(|e| e.to_string())?
        .configs()
        .clone();

    let all_auto = fan_indices.iter().all(|index| {
        configs
            .get(index)
            .is_none_or(|config| matches!(config, FanControlConfig::Auto))
    });

    let mut store = state.preset_store.lock().map_err(|e| e.to_string())?;
    store.active_preset = if all_auto {
        Some(AUTOMATIC_PRESET.to_string())
    } else {
        None
    };
    presets::save_preset_store(&store)
}

/// Reads sensors, overlays fan configs, emits `sensor_update`, and refreshes tray UI.
pub fn emit_fan_control_snapshot(app_handle: &AppHandle) {
    let mut service = SensorService::new();
    let Ok(mut sensor_data) = service.read_all_sensors() else {
        warn_log!("[fanguard] emit_fan_control_snapshot: sensor read failed");
        return;
    };

    let state = app_handle.state::<AppState>();
    if let Ok(writer_guard) = state.smc_writer.lock() {
        if let Some(writer) = writer_guard.as_deref() {
            if let Ok(mut control) = state.fan_control.lock() {
                if let Err(error) = control.tick(&sensor_data.details, &sensor_data.fans, writer) {
                    warn_log!("[fanguard] emit_fan_control_snapshot tick failed: {error}");
                }
                control.overlay_configs(&mut sensor_data.fans);
            }
        } else if let Ok(control) = state.fan_control.lock() {
            control.overlay_configs(&mut sensor_data.fans);
        }
    }

    if let Err(error) = app_handle.emit(SENSOR_UPDATE_EVENT, &sensor_data) {
        warn_log!("[fanguard] emit_fan_control_snapshot emit failed: {error}");
    }

    tray::update_tray_title(app_handle, &sensor_data);
    tray::update_tray_menu_force(app_handle, &sensor_data);
}

/// Applies overlay from in-memory fan control state without a full sensor rescan.
pub fn overlay_fan_control_on_sensors(state: &AppState, sensor_data: &mut crate::smc::SensorData) {
    if let Ok(control) = state.fan_control.lock() {
        control.overlay_configs(&mut sensor_data.fans);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fan_control::{FanControlConfig, FanControlState};
    use crate::smc::{FanData, FanMode};
    use crate::smc_writer::mock::MockSmcWriter;
    use std::sync::Mutex;

    fn make_fan(index: u8) -> FanData {
        FanData {
            index,
            label: format!("Fan {index}"),
            actual: 1200.0,
            min: 1200.0,
            max: 5800.0,
            target: 1200.0,
            mode: FanMode::Auto,
        }
    }

    fn test_state_with_control(control: FanControlState) -> AppState {
        AppState {
            fan_control: Mutex::new(control),
            smc_writer: Mutex::new(None),
            preset_store: Mutex::new(presets::PresetStore {
                active_preset: Some(AUTOMATIC_PRESET.to_string()),
                custom_presets: vec![],
            }),
            alert_config: Mutex::new(crate::alerts::AlertConfig {
                enabled: false,
                cpu_threshold: 90.0,
                cooldown_secs: 60,
            }),
            power_preset_config: Mutex::new(crate::power_presets::PowerPresetConfig {
                enabled: false,
                ac_preset: None,
                battery_preset: None,
            }),
            current_power_source: Mutex::new(crate::power_monitor::PowerSource::Unknown),
        }
    }

    #[test]
    fn sync_active_preset_clears_when_fan_is_forced() {
        let writer = MockSmcWriter::new();
        let fans = vec![make_fan(0)];
        let mut control = FanControlState::new();
        control
            .set_config(
                0,
                FanControlConfig::ConstantRpm { target_rpm: 4000.0 },
                &fans,
                &writer,
            )
            .expect("set constant rpm");
        let state = test_state_with_control(control);

        sync_active_preset(&state, &[0]).expect("sync should succeed");

        let store = state.preset_store.lock().expect("lock preset store");
        assert!(store.active_preset.is_none());
    }

    #[test]
    fn sync_active_preset_restores_automatic_when_all_auto() {
        let writer = MockSmcWriter::new();
        let mut control = FanControlState::new();
        control.set_auto(0, &writer).expect("set auto");
        let state = test_state_with_control(control);

        sync_active_preset(&state, &[0]).expect("sync should succeed");

        let store = state.preset_store.lock().expect("lock preset store");
        assert_eq!(store.active_preset.as_deref(), Some(AUTOMATIC_PRESET));
    }
}
