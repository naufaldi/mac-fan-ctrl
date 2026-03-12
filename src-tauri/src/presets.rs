//! Fan control preset system.
//!
//! Manages named presets that store per-fan control configurations.
//! Built-in presets: "Automatic" (all Auto), "Full Blast" (all max RPM).
//! Custom presets are persisted to `~/.config/fanguard/presets.json`.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::fan_control::FanControlConfig;
use crate::fs_util::fix_ownership_if_root;

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Preset {
    pub name: String,
    pub builtin: bool,
    pub configs: HashMap<u8, FanControlConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetStore {
    pub active_preset: Option<String>,
    pub custom_presets: Vec<Preset>,
}

impl Default for PresetStore {
    fn default() -> Self {
        Self {
            active_preset: Some("Automatic".to_string()),
            custom_presets: Vec::new(),
        }
    }
}

// ── Built-in presets ─────────────────────────────────────────────────────────

pub fn builtin_automatic() -> Preset {
    Preset {
        name: "Automatic".to_string(),
        builtin: true,
        configs: HashMap::new(), // empty = all fans Auto
    }
}

pub fn builtin_full_blast(fan_indices: &[u8], fan_maxes: &HashMap<u8, f32>) -> Preset {
    let configs = fan_indices
        .iter()
        .map(|idx| {
            let max_rpm = fan_maxes.get(idx).copied().unwrap_or(6000.0);
            (
                *idx,
                FanControlConfig::ConstantRpm {
                    target_rpm: max_rpm,
                },
            )
        })
        .collect();

    Preset {
        name: "Full Blast".to_string(),
        builtin: true,
        configs,
    }
}

pub fn builtin_sensor(fan_indices: &[u8]) -> Preset {
    let configs = fan_indices
        .iter()
        .map(|idx| {
            (
                *idx,
                FanControlConfig::SensorBased {
                    sensor_key: "TCPUAVG".to_string(),
                    temp_low: 40.0,
                    temp_high: 85.0,
                },
            )
        })
        .collect();

    Preset {
        name: "Sensor".to_string(),
        builtin: true,
        configs,
    }
}

// ── Persistence ──────────────────────────────────────────────────────────────

fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("fanguard")
}

fn presets_file() -> PathBuf {
    config_dir().join("presets.json")
}

pub fn load_preset_store() -> PresetStore {
    let path = presets_file();
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => PresetStore::default(),
    }
}

pub fn save_preset_store(store: &PresetStore) -> Result<(), String> {
    let path = presets_file();
    let dir = config_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create config dir: {e}"))?;
    let json =
        serde_json::to_string_pretty(store).map_err(|e| format!("Failed to serialize: {e}"))?;
    fs::write(&path, &json).map_err(|e| format!("Failed to write presets: {e}"))?;
    fix_ownership_if_root(&dir);
    fix_ownership_if_root(&path);
    Ok(())
}

// ── Preset operations ────────────────────────────────────────────────────────

pub fn all_presets(
    store: &PresetStore,
    fan_indices: &[u8],
    fan_maxes: &HashMap<u8, f32>,
) -> Vec<Preset> {
    let mut result = vec![
        builtin_automatic(),
        builtin_full_blast(fan_indices, fan_maxes),
        builtin_sensor(fan_indices),
    ];
    result.extend(store.custom_presets.clone());
    result
}

pub fn save_custom_preset(store: &mut PresetStore, preset: Preset) -> Result<(), String> {
    // Remove existing preset with same name (overwrite)
    store.custom_presets.retain(|p| p.name != preset.name);
    store.custom_presets.push(Preset {
        builtin: false,
        ..preset
    });
    save_preset_store(store)
}

pub fn delete_custom_preset(store: &mut PresetStore, name: &str) -> Result<(), String> {
    store.custom_presets.retain(|p| p.name != name);
    if store.active_preset.as_deref() == Some(name) {
        store.active_preset = Some("Automatic".to_string());
    }
    save_preset_store(store)
}

// ── Duplicate detection ──────────────────────────────────────────────────────

/// Returns the name of an existing preset whose configs match the given configs,
/// or `None` if no match is found. For the "Automatic" preset, an empty configs
/// map (all Auto) is considered a match.
pub fn find_preset_with_matching_configs(
    store: &PresetStore,
    configs: &HashMap<u8, FanControlConfig>,
    fan_indices: &[u8],
    fan_maxes: &HashMap<u8, f32>,
) -> Option<String> {
    let all = all_presets(store, fan_indices, fan_maxes);
    all.into_iter()
        .find(|preset| configs_match(&preset.configs, configs, fan_indices))
        .map(|preset| preset.name)
}

/// Two config maps match if every fan index resolves to the same effective config.
/// Missing entries are treated as `Auto`.
fn configs_match(
    a: &HashMap<u8, FanControlConfig>,
    b: &HashMap<u8, FanControlConfig>,
    fan_indices: &[u8],
) -> bool {
    let default = FanControlConfig::Auto;
    fan_indices
        .iter()
        .all(|idx| a.get(idx).unwrap_or(&default) == b.get(idx).unwrap_or(&default))
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_automatic_has_empty_configs() {
        let preset = builtin_automatic();
        assert_eq!(preset.name, "Automatic");
        assert!(preset.builtin);
        assert!(preset.configs.is_empty());
    }

    #[test]
    fn builtin_full_blast_sets_all_fans_to_max() {
        let indices = vec![0, 1];
        let mut maxes = HashMap::new();
        maxes.insert(0, 5800.0);
        maxes.insert(1, 6200.0);

        let preset = builtin_full_blast(&indices, &maxes);
        assert_eq!(preset.name, "Full Blast");
        assert!(preset.builtin);
        assert_eq!(preset.configs.len(), 2);

        match &preset.configs[&0] {
            FanControlConfig::ConstantRpm { target_rpm } => assert_eq!(*target_rpm, 5800.0),
            _ => panic!("Expected ConstantRpm"),
        }
        match &preset.configs[&1] {
            FanControlConfig::ConstantRpm { target_rpm } => assert_eq!(*target_rpm, 6200.0),
            _ => panic!("Expected ConstantRpm"),
        }
    }

    #[test]
    fn builtin_sensor_sets_all_fans_to_sensor_based() {
        let indices = vec![0, 1];
        let preset = builtin_sensor(&indices);
        assert_eq!(preset.name, "Sensor");
        assert!(preset.builtin);
        assert_eq!(preset.configs.len(), 2);

        match &preset.configs[&0] {
            FanControlConfig::SensorBased {
                sensor_key,
                temp_low,
                temp_high,
            } => {
                assert_eq!(sensor_key, "TCPUAVG");
                assert_eq!(*temp_low, 40.0);
                assert_eq!(*temp_high, 85.0);
            }
            _ => panic!("Expected SensorBased"),
        }
    }

    #[test]
    fn find_duplicate_detects_matching_automatic() {
        let store = PresetStore::default();
        let indices = vec![0];
        let mut maxes = HashMap::new();
        maxes.insert(0, 5800.0);

        // Empty configs = Automatic
        let configs = HashMap::new();
        let result = find_preset_with_matching_configs(&store, &configs, &indices, &maxes);
        assert_eq!(result, Some("Automatic".to_string()));
    }

    #[test]
    fn find_duplicate_detects_matching_full_blast() {
        let store = PresetStore::default();
        let indices = vec![0];
        let mut maxes = HashMap::new();
        maxes.insert(0, 5800.0);

        let mut configs = HashMap::new();
        configs.insert(
            0,
            FanControlConfig::ConstantRpm {
                target_rpm: 5800.0,
            },
        );
        let result = find_preset_with_matching_configs(&store, &configs, &indices, &maxes);
        assert_eq!(result, Some("Full Blast".to_string()));
    }

    #[test]
    fn find_duplicate_returns_none_for_unique_config() {
        let store = PresetStore::default();
        let indices = vec![0];
        let mut maxes = HashMap::new();
        maxes.insert(0, 5800.0);

        let mut configs = HashMap::new();
        configs.insert(
            0,
            FanControlConfig::ConstantRpm {
                target_rpm: 3000.0,
            },
        );
        let result = find_preset_with_matching_configs(&store, &configs, &indices, &maxes);
        assert_eq!(result, None);
    }

    #[test]
    fn all_presets_includes_builtins_and_custom() {
        let mut store = PresetStore::default();
        store.custom_presets.push(Preset {
            name: "Silent".to_string(),
            builtin: false,
            configs: HashMap::new(),
        });

        let indices = vec![0];
        let mut maxes = HashMap::new();
        maxes.insert(0, 5800.0);

        let presets = all_presets(&store, &indices, &maxes);
        assert_eq!(presets.len(), 4);
        assert_eq!(presets[0].name, "Automatic");
        assert_eq!(presets[1].name, "Full Blast");
        assert_eq!(presets[2].name, "Sensor");
        assert_eq!(presets[3].name, "Silent");
    }

    #[test]
    fn preset_store_serialization_roundtrip() {
        let store = PresetStore {
            active_preset: Some("Custom".to_string()),
            custom_presets: vec![Preset {
                name: "Custom".to_string(),
                builtin: false,
                configs: {
                    let mut m = HashMap::new();
                    m.insert(0, FanControlConfig::ConstantRpm { target_rpm: 3000.0 });
                    m
                },
            }],
        };

        let json = serde_json::to_string(&store).unwrap();
        let deserialized: PresetStore = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.active_preset, store.active_preset);
        assert_eq!(deserialized.custom_presets.len(), 1);
    }
}
