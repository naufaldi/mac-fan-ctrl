//! Fan control state manager.
//!
//! Tracks per-fan control configuration (Auto, Constant RPM, Sensor-based)
//! and drives the sensor-based interpolation loop on each poll tick.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::smc::{FanData, Sensor};
use crate::smc_writer::{SmcWriteError, SmcWriter};

// ── Emergency thermal threshold ──────────────────────────────────────────────

const EMERGENCY_TEMP_THRESHOLD: f64 = 95.0;

// ── Fan control configuration ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum FanControlConfig {
    Auto,
    ConstantRpm {
        target_rpm: f32,
    },
    SensorBased {
        sensor_key: String,
        temp_low: f32,
        temp_high: f32,
    },
}

// ── Fan control state ────────────────────────────────────────────────────────

pub struct FanControlState {
    configs: HashMap<u8, FanControlConfig>,
    emergency_active: bool,
}

impl FanControlState {
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
            emergency_active: false,
        }
    }

    /// Returns a snapshot of current configs for all fans.
    pub fn configs(&self) -> &HashMap<u8, FanControlConfig> {
        &self.configs
    }

    /// Returns whether the emergency thermal override is active.
    #[allow(dead_code)]
    pub fn is_emergency_active(&self) -> bool {
        self.emergency_active
    }

    /// Sets a fan's control configuration and immediately applies it.
    pub fn set_config(
        &mut self,
        fan_index: u8,
        config: FanControlConfig,
        fans: &[FanData],
        writer: &SmcWriter,
    ) -> Result<(), SmcWriteError> {
        apply_config(fan_index, &config, fans, writer)?;
        self.configs.insert(fan_index, config);
        Ok(())
    }

    /// Removes a fan's config (returns it to Auto) and applies.
    pub fn set_auto(
        &mut self,
        fan_index: u8,
        writer: &SmcWriter,
    ) -> Result<(), SmcWriteError> {
        writer.set_fan_auto(fan_index)?;
        self.configs.insert(fan_index, FanControlConfig::Auto);
        Ok(())
    }

    /// Called every poll cycle. For sensor-based fans, reads the linked
    /// sensor's current temperature and adjusts fan speed via linear
    /// interpolation. Also checks for emergency thermal conditions.
    pub fn tick(
        &mut self,
        sensors: &[Sensor],
        fans: &[FanData],
        writer: &SmcWriter,
    ) -> Result<(), SmcWriteError> {
        // Emergency check: if any sensor > threshold, force all fans to max
        let max_temp = sensors
            .iter()
            .filter(|s| s.unit == "C")
            .filter_map(|s| s.value)
            .fold(0.0_f64, f64::max);

        if max_temp >= EMERGENCY_TEMP_THRESHOLD {
            if !self.emergency_active {
                eprintln!(
                    "[mac-fan-ctrl] EMERGENCY: {max_temp:.1}°C >= {EMERGENCY_TEMP_THRESHOLD}°C — forcing all fans to max"
                );
                self.emergency_active = true;
                force_all_fans_max(fans, writer)?;
            }
            return Ok(());
        }

        // If emergency was active but temps dropped, restore configs
        if self.emergency_active {
            eprintln!(
                "[mac-fan-ctrl] Emergency cleared: {max_temp:.1}°C < {EMERGENCY_TEMP_THRESHOLD}°C — restoring configs"
            );
            self.emergency_active = false;
            for (fan_index, config) in &self.configs {
                let _ = apply_config(*fan_index, config, fans, writer);
            }
        }

        // Normal tick: update sensor-based fans
        for (fan_index, config) in &self.configs {
            if let FanControlConfig::SensorBased {
                sensor_key,
                temp_low,
                temp_high,
            } = config
            {
                let current_temp = sensors
                    .iter()
                    .find(|s| s.key == *sensor_key)
                    .and_then(|s| s.value);

                if let Some(temp) = current_temp {
                    let fan = fans.iter().find(|f| f.index == *fan_index);
                    if let Some(fan) = fan {
                        let target_rpm = interpolate_rpm(
                            temp as f32,
                            *temp_low,
                            *temp_high,
                            fan.min,
                            fan.max,
                        );
                        let _ = writer.set_fan_target_rpm(
                            *fan_index,
                            target_rpm,
                            fan.min,
                            fan.max,
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Restores all controlled fans to Auto mode.
    pub fn restore_all_auto(&mut self, writer: &SmcWriter) {
        for (fan_index, _) in &self.configs {
            if let Err(error) = writer.set_fan_auto(*fan_index) {
                eprintln!(
                    "[mac-fan-ctrl] Failed to restore fan {fan_index} to auto: {error}"
                );
            }
        }
        self.configs.clear();
        self.emergency_active = false;
    }
}

// ── Pure helper functions ────────────────────────────────────────────────────

/// Linearly interpolates fan RPM between min and max based on temperature.
///
/// - Below `temp_low`: returns `min_rpm`
/// - Above `temp_high`: returns `max_rpm`
/// - Between: linear interpolation
fn interpolate_rpm(
    current_temp: f32,
    temp_low: f32,
    temp_high: f32,
    min_rpm: f32,
    max_rpm: f32,
) -> f32 {
    if temp_high <= temp_low {
        return max_rpm;
    }

    let ratio = (current_temp - temp_low) / (temp_high - temp_low);
    let clamped = ratio.clamp(0.0, 1.0);
    min_rpm + clamped * (max_rpm - min_rpm)
}

/// Applies a single fan's config to SMC via the writer.
fn apply_config(
    fan_index: u8,
    config: &FanControlConfig,
    fans: &[FanData],
    writer: &SmcWriter,
) -> Result<(), SmcWriteError> {
    match config {
        FanControlConfig::Auto => writer.set_fan_auto(fan_index),
        FanControlConfig::ConstantRpm { target_rpm } => {
            let fan = fans
                .iter()
                .find(|f| f.index == fan_index)
                .ok_or(SmcWriteError::InvalidFanId(fan_index))?;
            let clamped = target_rpm.clamp(fan.min, fan.max);
            writer.set_fan_target_rpm(fan_index, clamped, fan.min, fan.max)
        }
        FanControlConfig::SensorBased { .. } => {
            // Sensor-based is handled by tick() — just ensure forced mode is set
            let fan = fans
                .iter()
                .find(|f| f.index == fan_index)
                .ok_or(SmcWriteError::InvalidFanId(fan_index))?;
            // Start at min RPM; tick() will adjust
            writer.set_fan_target_rpm(fan_index, fan.min, fan.min, fan.max)
        }
    }
}

/// Forces all fans to their maximum RPM (emergency mode).
fn force_all_fans_max(
    fans: &[FanData],
    writer: &SmcWriter,
) -> Result<(), SmcWriteError> {
    for fan in fans {
        writer.set_fan_target_rpm(fan.index, fan.max, fan.min, fan.max)?;
    }
    Ok(())
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolate_rpm_at_low_temp_returns_min() {
        let result = interpolate_rpm(30.0, 40.0, 80.0, 1200.0, 5800.0);
        assert_eq!(result, 1200.0);
    }

    #[test]
    fn interpolate_rpm_at_high_temp_returns_max() {
        let result = interpolate_rpm(90.0, 40.0, 80.0, 1200.0, 5800.0);
        assert_eq!(result, 5800.0);
    }

    #[test]
    fn interpolate_rpm_at_midpoint() {
        let result = interpolate_rpm(60.0, 40.0, 80.0, 1200.0, 5800.0);
        assert_eq!(result, 3500.0); // 1200 + 0.5 * (5800 - 1200)
    }

    #[test]
    fn interpolate_rpm_handles_equal_thresholds() {
        let result = interpolate_rpm(50.0, 50.0, 50.0, 1200.0, 5800.0);
        assert_eq!(result, 5800.0); // safety: return max
    }

    #[test]
    fn interpolate_rpm_at_quarter_point() {
        let result = interpolate_rpm(50.0, 40.0, 80.0, 1200.0, 5800.0);
        assert_eq!(result, 2350.0); // 1200 + 0.25 * 4600
    }

    #[test]
    fn new_state_has_no_configs() {
        let state = FanControlState::new();
        assert!(state.configs().is_empty());
        assert!(!state.is_emergency_active());
    }

    #[test]
    fn config_serialization_roundtrip() {
        let configs = vec![
            FanControlConfig::Auto,
            FanControlConfig::ConstantRpm { target_rpm: 2400.0 },
            FanControlConfig::SensorBased {
                sensor_key: "TC0P".to_string(),
                temp_low: 33.0,
                temp_high: 85.0,
            },
        ];

        for config in &configs {
            let json = serde_json::to_string(config).unwrap();
            let deserialized: FanControlConfig = serde_json::from_str(&json).unwrap();
            assert_eq!(*config, deserialized);
        }
    }
}
