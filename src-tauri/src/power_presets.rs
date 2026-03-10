//! Power source preset configuration and persistence.
//!
//! Allows users to assign different fan presets for AC vs battery power.
//! When the power source changes, the configured preset is auto-applied.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::fs_util::fix_ownership_if_root;

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerPresetConfig {
    pub enabled: bool,
    pub ac_preset: Option<String>,
    pub battery_preset: Option<String>,
}

impl Default for PowerPresetConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            ac_preset: None,
            battery_preset: None,
        }
    }
}

// ── Persistence ──────────────────────────────────────────────────────────────

fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("fanguard")
}

fn config_file() -> PathBuf {
    config_dir().join("power-presets.json")
}

pub fn load_power_preset_config() -> PowerPresetConfig {
    let path = config_file();
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => PowerPresetConfig::default(),
    }
}

pub fn save_power_preset_config(config: &PowerPresetConfig) -> Result<(), String> {
    let dir = config_dir();
    let path = config_file();
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create config dir: {e}"))?;
    let json =
        serde_json::to_string_pretty(config).map_err(|e| format!("Failed to serialize: {e}"))?;
    fs::write(&path, &json)
        .map_err(|e| format!("Failed to write power preset config: {e}"))?;
    fix_ownership_if_root(&dir);
    fix_ownership_if_root(&path);
    Ok(())
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_disabled() {
        let config = PowerPresetConfig::default();
        assert!(!config.enabled);
        assert!(config.ac_preset.is_none());
        assert!(config.battery_preset.is_none());
    }

    #[test]
    fn round_trip_serialization() {
        let config = PowerPresetConfig {
            enabled: true,
            ac_preset: Some("Performance".to_string()),
            battery_preset: Some("Silent".to_string()),
        };
        let json = serde_json::to_string(&config).expect("serialize");
        let parsed: PowerPresetConfig = serde_json::from_str(&json).expect("deserialize");
        assert!(parsed.enabled);
        assert_eq!(parsed.ac_preset.as_deref(), Some("Performance"));
        assert_eq!(parsed.battery_preset.as_deref(), Some("Silent"));
    }
}
