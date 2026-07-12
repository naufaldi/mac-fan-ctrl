//! Fan control state manager.
//!
//! Tracks per-fan control configuration (Auto, Constant RPM, Sensor-based)
//! and drives the sensor-based interpolation loop on each poll tick.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::log::warn_log;
use crate::smc::{FanData, FanMode, Sensor};
use crate::smc_writer::{SmcWriteApi, SmcWriteError};

// ── Safety constants ─────────────────────────────────────────────────────────

const EMERGENCY_TEMP_THRESHOLD: f64 = 95.0;
const EMERGENCY_CLEAR_THRESHOLD: f64 = 90.0;

/// Number of consecutive tick() cycles a sensor can be absent before
/// the linked fan is forced to max RPM for thermal safety.
const SENSOR_MISS_LIMIT: u32 = 3;

#[derive(Debug, Error)]
pub enum GlobalFanControlError {
    #[error("No fans were detected")]
    NoFans,
    #[error("Detected fans do not share a safe RPM range")]
    NoSharedRange,
    #[error("Target RPM {requested:.0} is outside the shared safe range {min_rpm:.0}-{max_rpm:.0}")]
    TargetOutOfRange {
        requested: f32,
        min_rpm: f32,
        max_rpm: f32,
    },
    #[error("Failed to apply {requested:.0} RPM to fan {fan_index}: {source}. Recovery: {recovery}")]
    ApplyFailed {
        requested: f32,
        fan_index: u8,
        source: SmcWriteError,
        recovery: String,
    },
    #[error("Failed to restore automatic fan control: {0}")]
    RestoreFailed(String),
}

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
    sensor_miss_counts: HashMap<u8, u32>,
}

impl FanControlState {
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
            emergency_active: false,
            sensor_miss_counts: HashMap::new(),
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
        writer: &dyn SmcWriteApi,
    ) -> Result<(), SmcWriteError> {
        apply_config(fan_index, &config, fans, writer)?;
        self.configs.insert(fan_index, config);
        Ok(())
    }

    pub fn set_all_constant_rpm(
        &mut self,
        target_rpm: f32,
        fans: &[FanData],
        writer: &dyn SmcWriteApi,
    ) -> Result<(), GlobalFanControlError> {
        if fans
            .iter()
            .any(|fan| !fan.min.is_finite() || !fan.max.is_finite() || fan.min > fan.max)
        {
            return Err(GlobalFanControlError::NoSharedRange);
        }
        let (min_rpm, max_rpm) = fans
            .iter()
            .map(|fan| (fan.min, fan.max))
            .reduce(|left, right| (left.0.max(right.0), left.1.min(right.1)))
            .ok_or(GlobalFanControlError::NoFans)?;

        if min_rpm > max_rpm {
            return Err(GlobalFanControlError::NoSharedRange);
        }
        if !target_rpm.is_finite() || target_rpm < min_rpm || target_rpm > max_rpm {
            return Err(GlobalFanControlError::TargetOutOfRange {
                requested: target_rpm,
                min_rpm,
                max_rpm,
            });
        }

        let apply_result = fans.iter().try_fold(Vec::<u8>::new(), |touched, fan| {
            let next_touched = touched
                .iter()
                .copied()
                .chain(std::iter::once(fan.index))
                .collect::<Vec<_>>();
            writer
                .set_fan_target_rpm(fan.index, target_rpm, fan.min, fan.max)
                .map(|()| next_touched.clone())
                .map_err(|source| (fan.index, source, next_touched))
        });

        match apply_result {
            Ok(touched) => {
                self.configs = touched
                    .into_iter()
                    .map(|fan_index| {
                        (fan_index, FanControlConfig::ConstantRpm { target_rpm })
                    })
                    .collect();
                Ok(())
            }
            Err((fan_index, source, touched)) => {
                let recovery = restore_fan_indices(&touched, writer);
                self.configs = recovery
                    .retry_indices
                    .iter()
                    .copied()
                    .map(|index| (index, FanControlConfig::ConstantRpm { target_rpm }))
                    .collect();
                Err(GlobalFanControlError::ApplyFailed {
                    requested: target_rpm,
                    fan_index,
                    source,
                    recovery: recovery.message,
                })
            }
        }
    }

    pub fn restore_detected_fans_auto(
        &mut self,
        fans: &[FanData],
        writer: &dyn SmcWriteApi,
    ) -> Result<(), GlobalFanControlError> {
        if fans.is_empty() {
            return Err(GlobalFanControlError::NoFans);
        }
        let fan_indices = fans.iter().map(|fan| fan.index).collect::<Vec<_>>();
        let recovery = restore_fan_indices(&fan_indices, writer);
        self.configs.retain(|index, _| recovery.retry_indices.contains(index));
        fans.iter()
            .filter(|fan| recovery.retry_indices.contains(&fan.index))
            .for_each(|fan| {
                self.configs.entry(fan.index).or_insert(
                    FanControlConfig::ConstantRpm {
                        target_rpm: fan.target,
                    },
                );
            });
        if recovery.retry_indices.is_empty() {
            Ok(())
        } else {
            Err(GlobalFanControlError::RestoreFailed(recovery.message))
        }
    }

    /// Removes a fan's config (returns it to Auto) and applies.
    /// Re-locks thermal control if no fans remain in a forced mode.
    pub fn set_auto(
        &mut self,
        fan_index: u8,
        writer: &dyn SmcWriteApi,
    ) -> Result<(), SmcWriteError> {
        writer.set_fan_auto(fan_index)?;
        self.configs.insert(fan_index, FanControlConfig::Auto);

        // Re-lock thermal enforcement if all fans are now auto
        let any_forced = self
            .configs
            .values()
            .any(|c| !matches!(c, FanControlConfig::Auto));
        if !any_forced {
            let _ = writer.lock_fan_control();
        }
        Ok(())
    }

    /// Called every poll cycle. For sensor-based fans, reads the linked
    /// sensor's current temperature and adjusts fan speed via linear
    /// interpolation. Also checks for emergency thermal conditions.
    pub fn tick(
        &mut self,
        sensors: &[Sensor],
        fans: &[FanData],
        writer: &dyn SmcWriteApi,
    ) -> Result<(), SmcWriteError> {
        // Collect readable temperature values
        let temp_readings: Vec<f64> = sensors
            .iter()
            .filter(|s| s.unit == "C")
            .filter_map(|s| s.value)
            .collect();

        // Emergency: total sensor failure — no readable temperatures at all
        let total_sensor_failure = temp_readings.is_empty();
        if total_sensor_failure {
            if !self.emergency_active {
                warn_log!(
                    "[fanguard] EMERGENCY: total sensor failure (0 readable temperature sensors) — forcing all fans to max"
                );
                self.emergency_active = true;
                force_all_fans_max(fans, writer)?;
            }
            return Ok(());
        }

        // Emergency: over-temperature
        let max_temp = temp_readings.iter().copied().fold(0.0_f64, f64::max);
        if max_temp >= EMERGENCY_TEMP_THRESHOLD {
            if !self.emergency_active {
                warn_log!(
                    "[fanguard] EMERGENCY: {max_temp:.1}°C >= {EMERGENCY_TEMP_THRESHOLD}°C — forcing all fans to max"
                );
                self.emergency_active = true;
                force_all_fans_max(fans, writer)?;
            }
            return Ok(());
        }

        // If emergency was active but temps dropped below clear threshold (hysteresis), restore configs
        if self.emergency_active && max_temp < EMERGENCY_CLEAR_THRESHOLD {
            warn_log!(
                "[fanguard] Emergency cleared: {max_temp:.1}°C < {EMERGENCY_CLEAR_THRESHOLD}°C — restoring configs"
            );
            self.emergency_active = false;
            self.sensor_miss_counts.clear();
            for (fan_index, config) in &self.configs {
                let _ = apply_config(*fan_index, config, fans, writer);
            }
        }

        // Normal tick: update sensor-based fans with miss tracking
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

                match current_temp {
                    Some(temp) => {
                        // Sensor found — reset miss counter and interpolate RPM
                        self.sensor_miss_counts.insert(*fan_index, 0);
                        if let Some(fan) = fans.iter().find(|f| f.index == *fan_index) {
                            let target_rpm = interpolate_rpm(
                                temp as f32,
                                *temp_low,
                                *temp_high,
                                fan.min,
                                fan.max,
                            );
                            let _ =
                                writer.set_fan_target_rpm(*fan_index, target_rpm, fan.min, fan.max);
                        }
                    }
                    None => {
                        // Sensor missing this cycle — track consecutive misses
                        let misses = self.sensor_miss_counts.entry(*fan_index).or_insert(0);
                        *misses += 1;

                        if *misses >= SENSOR_MISS_LIMIT {
                            warn_log!(
                                "[fanguard] SAFETY: sensor '{sensor_key}' missing for {misses} cycles on fan {fan_index} — forcing fan to max RPM"
                            );
                            if let Some(fan) = fans.iter().find(|f| f.index == *fan_index) {
                                let _ = writer
                                    .set_fan_target_rpm(*fan_index, fan.max, fan.min, fan.max);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Overlays active fan control configs onto raw SMC fan data.
    ///
    /// Apple Silicon often reads back `target = 0` from SMC even after a
    /// successful write. This method patches the emitted `FanData` so the
    /// frontend displays the user's configured values instead of stale SMC
    /// readbacks.
    pub fn overlay_configs(&self, fans: &mut [FanData]) {
        for fan in fans.iter_mut() {
            if let Some(config) = self.configs.get(&fan.index) {
                match config {
                    FanControlConfig::Auto => {
                        fan.mode = FanMode::Auto;
                    }
                    FanControlConfig::ConstantRpm { target_rpm } => {
                        fan.mode = FanMode::Forced;
                        fan.target = *target_rpm;
                    }
                    FanControlConfig::SensorBased { .. } => {
                        fan.mode = FanMode::Forced;
                        // target is continuously updated by tick(); keep current value
                    }
                }
            }
        }
    }

    /// Restores all controlled fans to Auto mode and re-locks
    /// thermal enforcement.
    pub fn restore_all_auto(&mut self, writer: &dyn SmcWriteApi) {
        for fan_index in self.configs.keys() {
            if let Err(error) = writer.set_fan_auto(*fan_index) {
                warn_log!("[fanguard] Failed to restore fan {fan_index} to auto: {error}");
            }
        }
        self.configs.clear();
        self.emergency_active = false;
        let _ = writer.lock_fan_control();
    }
}

struct RecoveryOutcome {
    retry_indices: Vec<u8>,
    message: String,
}

fn restore_fan_indices(fan_indices: &[u8], writer: &dyn SmcWriteApi) -> RecoveryOutcome {
    let failures = fan_indices
        .iter()
        .filter_map(|fan_index| {
            writer
                .set_fan_auto(*fan_index)
                .err()
                .map(|error| (*fan_index, format!("fan {fan_index}: {error}")))
        })
        .collect::<Vec<_>>();
    let lock_failure = writer.lock_fan_control().err();
    let all_failures = failures
		.iter()
		.map(|(_, message)| message.clone())
        .chain(lock_failure.map(|error| format!("thermal-control lock: {error}")))
        .collect::<Vec<_>>();
	let retry_indices = if all_failures.is_empty() {
		Vec::new()
	} else if failures.is_empty() {
		fan_indices.to_vec()
    } else {
		failures.iter().map(|(fan_index, _)| *fan_index).collect()
	};
	RecoveryOutcome {
		retry_indices,
		message: if all_failures.is_empty() {
			"all touched fans restored to Auto".to_string()
		} else {
			all_failures.join("; ")
		},
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
    writer: &dyn SmcWriteApi,
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
fn force_all_fans_max(fans: &[FanData], writer: &dyn SmcWriteApi) -> Result<(), SmcWriteError> {
    for fan in fans {
        writer.set_fan_target_rpm(fan.index, fan.max, fan.min, fan.max)?;
    }
    Ok(())
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::smc::{NullReason, SensorSource};
    use crate::smc_writer::mock::{MockSmcCall, MockSmcWriter};

	struct FailOnFanWriter {
		calls: std::cell::RefCell<Vec<MockSmcCall>>,
		failed_fan: u8,
		fail_auto_for: Option<u8>,
	}

	unsafe impl Send for FailOnFanWriter {}
	unsafe impl Sync for FailOnFanWriter {}

	impl SmcWriteApi for FailOnFanWriter {
		fn set_fan_target_rpm(
			&self,
			fan_index: u8,
			rpm: f32,
			_min_rpm: f32,
			_max_rpm: f32,
		) -> Result<(), SmcWriteError> {
			self.calls.borrow_mut().push(MockSmcCall::SetFanTargetRpm { fan_index, rpm });
			if fan_index == self.failed_fan {
				Err(SmcWriteError::TargetVerificationFailed {
					fan_index,
					requested: rpm,
					readback: 0.0,
				})
			} else {
				Ok(())
			}
		}

		fn set_fan_auto(&self, fan_index: u8) -> Result<(), SmcWriteError> {
			self.calls.borrow_mut().push(MockSmcCall::SetFanAuto { fan_index });
			if self.fail_auto_for == Some(fan_index) {
				Err(SmcWriteError::InsufficientPrivileges)
			} else {
				Ok(())
			}
		}

		fn lock_fan_control(&self) -> Result<(), SmcWriteError> {
			self.calls.borrow_mut().push(MockSmcCall::LockFanControl);
			Ok(())
		}

		fn unlock_fan_control(&self) -> Result<(), SmcWriteError> {
			Ok(())
		}

		fn diagnose_fan_control(&self) -> Vec<String> {
			Vec::new()
		}
	}

    fn make_fan(index: u8, min: f32, max: f32) -> FanData {
        FanData {
            index,
            label: format!("Fan {index}"),
            actual: min,
            min,
            max,
            target: min,
            mode: FanMode::Auto,
        }
    }

    fn make_sensor(key: &str, value: Option<f64>) -> Sensor {
        Sensor {
            key: key.to_string(),
            name: key.to_string(),
            value,
            unit: "C".to_string(),
            sensor_type: "temperature".to_string(),
            source: SensorSource::Smc,
            null_reason: if value.is_none() {
                Some(NullReason::ReadError)
            } else {
                None
            },
        }
    }

    // ── Interpolation tests ──────────────────────────────────────────────

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
        assert_eq!(result, 3500.0);
    }

    #[test]
    fn interpolate_rpm_handles_equal_thresholds() {
        let result = interpolate_rpm(50.0, 50.0, 50.0, 1200.0, 5800.0);
        assert_eq!(result, 5800.0);
    }

    #[test]
    fn interpolate_rpm_at_quarter_point() {
        let result = interpolate_rpm(50.0, 40.0, 80.0, 1200.0, 5800.0);
        assert_eq!(result, 2350.0);
    }

    // ── State tests ──────────────────────────────────────────────────────

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

    // ── Emergency: total sensor failure (#9) ─────────────────────────────

    #[test]
    fn tick_triggers_emergency_when_sensor_list_is_empty() {
        let writer = MockSmcWriter::new();
        let mut state = FanControlState::new();
        let fans = vec![make_fan(0, 1200.0, 5800.0)];

        let result = state.tick(&[], &fans, &writer);
        assert!(result.is_ok());
        assert!(state.is_emergency_active());

        let calls = writer.calls.borrow();
        assert!(calls.iter().any(|c| matches!(
            c,
            MockSmcCall::SetFanTargetRpm { fan_index: 0, rpm } if (*rpm - 5800.0).abs() < 1.0
        )));
    }

    #[test]
    fn tick_triggers_emergency_when_all_sensor_values_are_none() {
        let writer = MockSmcWriter::new();
        let mut state = FanControlState::new();
        let fans = vec![make_fan(0, 1200.0, 5800.0)];
        let sensors = vec![make_sensor("TC0P", None)];

        let result = state.tick(&sensors, &fans, &writer);
        assert!(result.is_ok());
        assert!(state.is_emergency_active());
    }

    #[test]
    fn tick_clears_emergency_when_sensors_return() {
        let writer = MockSmcWriter::new();
        let mut state = FanControlState::new();
        let fans = vec![make_fan(0, 1200.0, 5800.0)];

        // Trigger emergency
        state.tick(&[], &fans, &writer).unwrap();
        assert!(state.is_emergency_active());

        // Sensors return with normal temps
        let sensors = vec![make_sensor("TC0P", Some(50.0))];
        state.tick(&sensors, &fans, &writer).unwrap();
        assert!(!state.is_emergency_active());
    }

    #[test]
    fn tick_triggers_emergency_on_over_temperature() {
        let writer = MockSmcWriter::new();
        let mut state = FanControlState::new();
        let fans = vec![make_fan(0, 1200.0, 5800.0)];
        let sensors = vec![make_sensor("TC0P", Some(96.0))];

        state.tick(&sensors, &fans, &writer).unwrap();
        assert!(state.is_emergency_active());
    }

    // ── Thermal hysteresis (#19) ───────────────────────────────────────────

    #[test]
    fn emergency_stays_active_in_hysteresis_band() {
        let writer = MockSmcWriter::new();
        let mut state = FanControlState::new();
        let fans = vec![make_fan(0, 1200.0, 5800.0)];

        // Trigger emergency at 96°C
        let hot = vec![make_sensor("TC0P", Some(96.0))];
        state.tick(&hot, &fans, &writer).unwrap();
        assert!(state.is_emergency_active());

        // Drop to 92°C (between CLEAR=90 and TRIGGER=95) — should stay active
        let warm = vec![make_sensor("TC0P", Some(92.0))];
        state.tick(&warm, &fans, &writer).unwrap();
        assert!(
            state.is_emergency_active(),
            "Emergency should remain active in hysteresis band (90–95°C)"
        );
    }

    #[test]
    fn emergency_clears_below_hysteresis_threshold() {
        let writer = MockSmcWriter::new();
        let mut state = FanControlState::new();
        let fans = vec![make_fan(0, 1200.0, 5800.0)];

        // Trigger emergency at 96°C
        let hot = vec![make_sensor("TC0P", Some(96.0))];
        state.tick(&hot, &fans, &writer).unwrap();
        assert!(state.is_emergency_active());

        // Drop to 89°C (below CLEAR=90) — should clear
        let cool = vec![make_sensor("TC0P", Some(89.0))];
        state.tick(&cool, &fans, &writer).unwrap();
        assert!(
            !state.is_emergency_active(),
            "Emergency should clear below 90°C"
        );
    }

    // ── Sensor disappearance (#8) ────────────────────────────────────────

    #[test]
    fn tick_forces_fan_max_when_sensor_absent_for_limit_cycles() {
        let writer = MockSmcWriter::new();
        let mut state = FanControlState::new();
        let fans = vec![make_fan(0, 1200.0, 5800.0)];
        let config = FanControlConfig::SensorBased {
            sensor_key: "TC0P".to_string(),
            temp_low: 40.0,
            temp_high: 80.0,
        };
        state.configs.insert(0, config);

        // Provide some sensor to avoid total sensor failure, but NOT TC0P
        let sensors = vec![make_sensor("OTHER", Some(50.0))];

        // Tick SENSOR_MISS_LIMIT times with missing sensor
        for _ in 0..SENSOR_MISS_LIMIT {
            state.tick(&sensors, &fans, &writer).unwrap();
        }

        let calls = writer.calls.borrow();
        // On the 3rd miss, fan should be forced to max
        assert!(calls.iter().any(|c| matches!(
            c,
            MockSmcCall::SetFanTargetRpm { fan_index: 0, rpm } if (*rpm - 5800.0).abs() < 1.0
        )));
    }

    #[test]
    fn tick_resets_miss_count_on_successful_sensor_read() {
        let writer = MockSmcWriter::new();
        let mut state = FanControlState::new();
        let fans = vec![make_fan(0, 1200.0, 5800.0)];
        let config = FanControlConfig::SensorBased {
            sensor_key: "TC0P".to_string(),
            temp_low: 40.0,
            temp_high: 80.0,
        };
        state.configs.insert(0, config);

        let other_sensors = vec![make_sensor("OTHER", Some(50.0))];
        let good_sensors = vec![
            make_sensor("OTHER", Some(50.0)),
            make_sensor("TC0P", Some(60.0)),
        ];

        // Miss twice (below limit)
        state.tick(&other_sensors, &fans, &writer).unwrap();
        state.tick(&other_sensors, &fans, &writer).unwrap();

        // Sensor returns — should reset counter
        state.tick(&good_sensors, &fans, &writer).unwrap();

        // Miss twice again — should NOT trigger because counter was reset
        state.tick(&other_sensors, &fans, &writer).unwrap();
        state.tick(&other_sensors, &fans, &writer).unwrap();

        // No max-RPM call should have been made (only interpolated RPM calls)
        let calls = writer.calls.borrow();
        let max_rpm_calls: Vec<_> = calls
            .iter()
            .filter(|c| {
                matches!(
                    c,
                    MockSmcCall::SetFanTargetRpm { rpm, .. } if (*rpm - 5800.0).abs() < 1.0
                )
            })
            .collect();
        assert!(
            max_rpm_calls.is_empty(),
            "Expected no max-RPM calls, got {max_rpm_calls:?}"
        );
    }

    // ── set_config / set_auto with mock ──────────────────────────────────

    #[test]
    fn set_config_constant_rpm_writes_to_mock() {
        let writer = MockSmcWriter::new();
        let mut state = FanControlState::new();
        let fans = vec![make_fan(0, 1200.0, 5800.0)];

        let result = state.set_config(
            0,
            FanControlConfig::ConstantRpm { target_rpm: 3000.0 },
            &fans,
            &writer,
        );
        assert!(result.is_ok());
        assert_eq!(state.configs().len(), 1);

        let calls = writer.calls.borrow();
        assert!(calls.iter().any(|c| matches!(
            c,
            MockSmcCall::SetFanTargetRpm { fan_index: 0, rpm } if (*rpm - 3000.0).abs() < 1.0
        )));
    }

    #[test]
    fn set_auto_restores_fan_and_locks_when_all_auto() {
        let writer = MockSmcWriter::new();
        let mut state = FanControlState::new();
        let fans = vec![make_fan(0, 1200.0, 5800.0)];

        // Set a config first
        state
            .set_config(
                0,
                FanControlConfig::ConstantRpm { target_rpm: 3000.0 },
                &fans,
                &writer,
            )
            .unwrap();

        // Now set back to auto
        state.set_auto(0, &writer).unwrap();

        let calls = writer.calls.borrow();
        assert!(calls
            .iter()
            .any(|c| matches!(c, MockSmcCall::SetFanAuto { fan_index: 0 })));
        assert!(calls
            .iter()
            .any(|c| matches!(c, MockSmcCall::LockFanControl)));
    }

    #[test]
    fn overlay_configs_sets_forced_mode_and_target_for_constant_rpm() {
        let mut state = FanControlState::new();
        state
            .configs
            .insert(0, FanControlConfig::ConstantRpm { target_rpm: 4000.0 });
        let mut fans = vec![make_fan(0, 1200.0, 5800.0)];

        state.overlay_configs(&mut fans);

        assert_eq!(fans[0].mode, FanMode::Forced);
        assert_eq!(fans[0].target, 4000.0);
    }

    #[test]
    fn overlay_configs_sets_auto_mode_for_auto_config() {
        let mut state = FanControlState::new();
        state.configs.insert(0, FanControlConfig::Auto);
        let mut fans = vec![make_fan(0, 1200.0, 5800.0)];
        fans[0].mode = FanMode::Forced;
        fans[0].target = 5000.0;

        state.overlay_configs(&mut fans);

        assert_eq!(fans[0].mode, FanMode::Auto);
    }

    #[test]
    fn overlay_configs_leaves_unconfigured_fan_unchanged() {
        let state = FanControlState::new();
        let mut fans = vec![make_fan(0, 1200.0, 5800.0)];
        fans[0].mode = FanMode::Forced;
        fans[0].target = 5000.0;

        state.overlay_configs(&mut fans);

        assert_eq!(fans[0].mode, FanMode::Forced);
        assert_eq!(fans[0].target, 5000.0);
    }

	#[test]
	fn set_all_constant_rpm_uses_shared_range_and_updates_every_fan() {
		let writer = MockSmcWriter::new();
		let mut state = FanControlState::new();
		let fans = vec![make_fan(0, 1200.0, 5800.0), make_fan(1, 1600.0, 6200.0)];

		state.set_all_constant_rpm(3200.0, &fans, &writer).unwrap();

		assert_eq!(state.configs().len(), 2);
		let calls = writer.calls.borrow();
		assert_eq!(
			calls.iter().filter(|call| matches!(call, MockSmcCall::SetFanTargetRpm { rpm, .. } if (*rpm - 3200.0).abs() < 1.0)).count(),
			2,
		);
	}

	#[test]
	fn set_all_constant_rpm_rejects_target_outside_shared_range() {
		let writer = MockSmcWriter::new();
		let mut state = FanControlState::new();
		let fans = vec![make_fan(0, 1200.0, 5800.0), make_fan(1, 1600.0, 6200.0)];

		let result = state.set_all_constant_rpm(1400.0, &fans, &writer);

		assert!(matches!(result, Err(GlobalFanControlError::TargetOutOfRange { min_rpm: 1600.0, max_rpm: 5800.0, .. })));
		assert!(writer.calls.borrow().is_empty());
	}

	#[test]
	fn set_all_constant_rpm_restores_touched_fans_when_a_later_write_fails() {
		let writer = FailOnFanWriter {
			calls: std::cell::RefCell::new(Vec::new()),
			failed_fan: 1,
			fail_auto_for: None,
		};
		let mut state = FanControlState::new();
		let fans = vec![make_fan(0, 1200.0, 5800.0), make_fan(1, 1200.0, 5800.0)];

		let result = state.set_all_constant_rpm(3200.0, &fans, &writer);

		assert!(matches!(result, Err(GlobalFanControlError::ApplyFailed { fan_index: 1, .. })));
		assert!(state.configs().is_empty());
		let calls = writer.calls.borrow();
		assert!(calls.iter().any(|call| matches!(call, MockSmcCall::SetFanAuto { fan_index: 0 })));
		assert!(calls.iter().any(|call| matches!(call, MockSmcCall::SetFanAuto { fan_index: 1 })));
		assert!(calls.iter().any(|call| matches!(call, MockSmcCall::LockFanControl)));
	}

	#[test]
	fn set_all_constant_rpm_keeps_failed_recovery_tracked_for_shutdown_retry() {
		let writer = FailOnFanWriter {
			calls: std::cell::RefCell::new(Vec::new()),
			failed_fan: 1,
			fail_auto_for: Some(0),
		};
		let mut state = FanControlState::new();
		let fans = vec![make_fan(0, 1200.0, 5800.0), make_fan(1, 1200.0, 5800.0)];

		let result = state.set_all_constant_rpm(3200.0, &fans, &writer);

		assert!(matches!(result, Err(GlobalFanControlError::ApplyFailed { .. })));
		assert!(matches!(
			state.configs().get(&0),
			Some(FanControlConfig::ConstantRpm { target_rpm: 3200.0 })
		));
		assert!(!state.configs().contains_key(&1));
	}
}
