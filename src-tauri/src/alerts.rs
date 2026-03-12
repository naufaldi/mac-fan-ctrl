//! Temperature alert configuration and persistence.
//!
//! Stores user-configurable alert thresholds for temperature monitoring.
//! When a sensor exceeds its threshold, a native macOS notification fires
//! (with a cooldown to prevent spam).

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::fs_util::fix_ownership_if_root;

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    pub enabled: bool,
    pub cpu_threshold: f64,
    pub cooldown_secs: u64,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cpu_threshold: 85.0,
            cooldown_secs: 300,
        }
    }
}

// ── Persistence ──────────────────────────────────────────────────────────────

fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("fanguard")
}

fn alerts_file() -> PathBuf {
    config_dir().join("alerts.json")
}

pub fn load_alert_config() -> AlertConfig {
    let path = alerts_file();
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => AlertConfig::default(),
    }
}

pub fn save_alert_config(config: &AlertConfig) -> Result<(), String> {
    let dir = config_dir();
    let path = alerts_file();
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create config dir: {e}"))?;
    let json =
        serde_json::to_string_pretty(config).map_err(|e| format!("Failed to serialize: {e}"))?;
    fs::write(&path, &json).map_err(|e| format!("Failed to write alerts config: {e}"))?;
    fix_ownership_if_root(&dir);
    fix_ownership_if_root(&path);
    Ok(())
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_sane_values() {
        let config = AlertConfig::default();
        assert!(config.enabled);
        assert!((config.cpu_threshold - 85.0).abs() < f64::EPSILON);
        assert_eq!(config.cooldown_secs, 300);
    }

    #[test]
    fn round_trip_serialization() {
        let config = AlertConfig {
            enabled: false,
            cpu_threshold: 90.0,
            cooldown_secs: 600,
        };
        let json = serde_json::to_string(&config).expect("serialize");
        let parsed: AlertConfig = serde_json::from_str(&json).expect("deserialize");
        assert!(!parsed.enabled);
        assert!((parsed.cpu_threshold - 90.0).abs() < f64::EPSILON);
        assert_eq!(parsed.cooldown_secs, 600);
    }
}
