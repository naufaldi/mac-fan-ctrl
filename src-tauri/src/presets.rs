//! Fan control preset system.
//!
//! Manages named presets that store per-fan control configurations.
//! Built-in presets: "Automatic" (all Auto), "Full Blast" (all max RPM).
//! Custom presets are persisted to `~/.config/mac-fan-ctrl/presets.json`.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::fan_control::FanControlConfig;

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
        .filter_map(|idx| {
            let max_rpm = fan_maxes.get(idx).copied().unwrap_or(6000.0);
            Some((
                *idx,
                FanControlConfig::ConstantRpm {
                    target_rpm: max_rpm,
                },
            ))
        })
        .collect();

    Preset {
        name: "Full Blast".to_string(),
        builtin: true,
        configs,
    }
}

// ── Persistence ──────────────────────────────────────────────────────────────

fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("mac-fan-ctrl")
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
    fs::write(&path, json).map_err(|e| format!("Failed to write presets: {e}"))
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
        assert_eq!(presets.len(), 3);
        assert_eq!(presets[0].name, "Automatic");
        assert_eq!(presets[1].name, "Full Blast");
        assert_eq!(presets[2].name, "Silent");
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
