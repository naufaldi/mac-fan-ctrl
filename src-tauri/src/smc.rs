use std::collections::HashMap;
use std::process::Command;

use macsmc::{DataValue, Smc};
use serde::Serialize;
use thiserror::Error;

use crate::apple_silicon_sensors::read_apple_silicon_sensors;

#[derive(Debug, Error)]
pub enum SmcError {
    #[error("SMC connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Failed to read sensor: {0}")]
    ReadFailed(String),
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Sensor {
    pub key: String,
    pub name: String,
    pub value: Option<f64>,
    pub unit: String,
    pub sensor_type: String,
    pub source: SensorSource,
    pub null_reason: Option<NullReason>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SensorSource {
    Smc,
    IohidIokit,
    Derived,
    Placeholder,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NullReason {
    Placeholder,
    Unsupported,
    ReadError,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SummarySensors {
    pub cpu_package: Option<Sensor>,
    pub gpu: Option<Sensor>,
    pub ram: Option<Sensor>,
    pub ssd: Option<Sensor>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SensorData {
    pub summary: SummarySensors,
    pub details: Vec<Sensor>,
    pub diagnostics: SensorDiagnostics,
}

pub struct SmcClient {
    smc: Smc,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct SensorDiagnostics {
    pub model_id: Option<String>,
    pub diagnostics_enabled: bool,
    pub active_providers: Vec<String>,
    pub unresolved: Vec<UnresolvedSensor>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct UnresolvedSensor {
    pub provider: String,
    pub raw_name: String,
    pub reason: String,
}

pub struct SensorService {
    smc_client: Option<SmcClient>,
    diagnostics_enabled: bool,
    model_id: Option<String>,
    use_apple_silicon_provider: bool,
}

fn sensor_type_order(sensor_type: &str) -> usize {
    match sensor_type {
        "Cpu" => 0,
        "Gpu" => 1,
        "Memory" => 2,
        "Storage" => 3,
        "Battery" => 4,
        "Power" => 5,
        "Trackpad" => 6,
        _ => 7,
    }
}

fn create_sensor(
    key: &str,
    name: &str,
    value: Option<f64>,
    unit: &str,
    sensor_type: &str,
    source: SensorSource,
    null_reason: Option<NullReason>,
) -> Sensor {
    Sensor {
        key: key.to_string(),
        name: name.to_string(),
        value,
        unit: unit.to_string(),
        sensor_type: sensor_type.to_string(),
        source,
        null_reason,
    }
}

fn maybe_temp_sensor(key: &str, name: &str, value: f32, sensor_type: &str) -> Option<Sensor> {
    (value > 0.0).then(|| {
        create_sensor(
            key,
            name,
            Some(value as f64),
            "C",
            sensor_type,
            SensorSource::Smc,
            None,
        )
    })
}

fn maybe_power_sensor(key: &str, name: &str, value: f32) -> Option<Sensor> {
    (value > 0.0).then(|| {
        create_sensor(
            key,
            name,
            Some(value as f64),
            "W",
            "Power",
            SensorSource::Smc,
            None,
        )
    })
}

fn to_cpu_average_sensor(values: &[f64]) -> Option<Sensor> {
    if values.is_empty() {
        return None;
    }

    Some(create_sensor(
        "TCPUAVG",
        "CPU Core Average",
        Some(values.iter().sum::<f64>() / values.len() as f64),
        "C",
        "Cpu",
        SensorSource::Derived,
        None,
    ))
}

fn estimated_efficiency_core_count(total_cores: usize) -> usize {
    if total_cores >= 8 {
        2
    } else {
        0
    }
}

fn to_cpu_core_label(index: usize, efficiency_cores: usize) -> String {
    if index < efficiency_cores {
        format!("CPU Efficiency Core {}", index + 1)
    } else {
        format!("CPU Performance Core {}", index - efficiency_cores + 1)
    }
}

fn has_sensor_type(details: &[Sensor], sensor_type: &str) -> bool {
    details.iter().any(|sensor| sensor.sensor_type == sensor_type)
}

fn has_sensor_named(details: &[Sensor], name: &str) -> bool {
    details.iter().any(|sensor| sensor.name == name)
}

fn ensure_reference_placeholders(details: Vec<Sensor>) -> Vec<Sensor> {
    let placeholder_specs = [
        ("DISK_SECTION", "Disk Drives:", "C", "Storage"),
        ("SSD", "APPLE SSD", "C", "Storage"),
        ("TRKP", "Trackpad", "C", "Trackpad"),
        ("TM0P", "Memory Bank 1", "C", "Memory"),
        ("TM1P", "Memory Bank 2", "C", "Memory"),
        ("TB0T", "Battery", "C", "Battery"),
        ("TB1T", "Battery Gas Gauge", "C", "Battery"),
        ("TPCD", "Power Manager Die Average", "C", "Power"),
        ("PDTR", "Power Supply Proximity", "W", "Power"),
    ];

    let placeholders = placeholder_specs
        .into_iter()
        .filter(|(key, name, _unit, sensor_type)| {
            !has_sensor_type(&details, sensor_type) && !has_sensor_named(&details, name) && !details.iter().any(|sensor| sensor.key == *key)
        })
        .map(|(key, name, unit, sensor_type)| {
            create_sensor(
                key,
                name,
                None,
                unit,
                sensor_type,
                SensorSource::Placeholder,
                Some(NullReason::Placeholder),
            )
        })
        .collect::<Vec<_>>();

    let mut merged = details;
    merged.extend(placeholders);
    merged
}

pub(crate) fn sort_sensors_for_display(mut details: Vec<Sensor>) -> Vec<Sensor> {
    details.sort_by(|left, right| {
        let type_cmp = sensor_type_order(&left.sensor_type).cmp(&sensor_type_order(&right.sensor_type));
        if type_cmp != std::cmp::Ordering::Equal {
            return type_cmp;
        }

        left.name.cmp(&right.name)
    });
    details
}

fn pick_summary_sensor(details: &[Sensor], preferred_keys: &[&str], sensor_type: &str) -> Option<Sensor> {
    preferred_keys
        .iter()
        .find_map(|key| details.iter().find(|sensor| sensor.key == *key).cloned())
        .or_else(|| details.iter().find(|sensor| sensor.sensor_type == sensor_type && sensor.value.is_some()).cloned())
        .or_else(|| details.iter().find(|sensor| sensor.sensor_type == sensor_type).cloned())
}

pub(crate) fn build_summary(details: &[Sensor]) -> SummarySensors {
    let cpu_package = pick_summary_sensor(details, &["TCPUAVG", "TC0D", "TC0P"], "Cpu");
    let gpu = pick_summary_sensor(details, &["TGAVG", "TG0D", "TG0P"], "Gpu");
    let ram = pick_summary_sensor(details, &["TM0P"], "Memory");
    let ssd = pick_summary_sensor(details, &["SSD", "TH0P", "TH1P"], "Storage").or_else(|| {
        Some(create_sensor(
            "SSD",
            "APPLE SSD",
            None,
            "C",
            "Storage",
            SensorSource::Placeholder,
            Some(NullReason::Placeholder),
        ))
    });

    SummarySensors {
        cpu_package,
        gpu,
        ram,
        ssd,
    }
}

impl SmcClient {
    pub fn new() -> Result<Self, SmcError> {
        let smc = Smc::connect()
            .map_err(|e| SmcError::ConnectionFailed(e.to_string()))?;
        Ok(Self { smc })
    }

    fn read_smc_details(&mut self) -> Vec<Sensor> {
        let baseline = [
            self.read_cpu_temperature_sensors(),
            self.read_cpu_core_sensors(),
            self.read_gpu_temperature_sensors(),
            self.read_other_temperature_sensors(),
            self.read_battery_sensors(),
            self.read_power_sensors(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        let dynamic = self.read_dynamic_temperature_sensors();
        let derived = derive_apple_silicon_catalog_rows(&dynamic);

        [baseline, dynamic, derived]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
    }

    fn read_dynamic_temperature_sensors(&mut self) -> Vec<Sensor> {
        let all_data = match self.smc.all_data() {
            Ok(data) => data,
            Err(error) => {
                eprintln!("[mac-fan-ctrl] Failed reading dynamic SMC keys: {error}");
                return Vec::new();
            }
        };

        all_data
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let value = entry.value.ok().flatten().and_then(data_value_to_f64)?;
                if !(1.0..=130.0).contains(&value) {
                    return None;
                }
                let key = entry.key;
                if !key.starts_with('T') {
                    return None;
                }
                let (name, sensor_type) = classify_dynamic_temperature_key(&key);
                Some(create_sensor(
                    &key,
                    &name,
                    Some(value),
                    "C",
                    sensor_type,
                    SensorSource::Smc,
                    None,
                ))
            })
            .collect::<Vec<_>>()
    }

    fn read_cpu_temperature_sensors(&mut self) -> Vec<Sensor> {
        match self.smc.cpu_temperature() {
            Ok(cpu_temps) => [
                maybe_temp_sensor("TC0D", "CPU Die", *cpu_temps.die, "Cpu"),
                maybe_temp_sensor("TC0P", "CPU Proximity", *cpu_temps.proximity, "Cpu"),
                maybe_temp_sensor("TCGC", "CPU Graphics", *cpu_temps.graphics, "Cpu"),
                maybe_temp_sensor("TCSA", "CPU System Agent", *cpu_temps.system_agent, "Memory"),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>(),
            Err(error) => {
                eprintln!("[mac-fan-ctrl] Failed reading CPU temperatures: {error}");
                Vec::new()
            }
        }
    }

    fn read_cpu_core_sensors(&mut self) -> Vec<Sensor> {
        match self.smc.cpu_core_temps() {
            Ok(core_temps) => {
                let values = core_temps
                    .enumerate()
                    .filter_map(|(index, celsius)| celsius.ok().map(|value| (index, *value)))
                    .filter(|(_, value)| *value > 0.0)
                    .collect::<Vec<_>>();

                let efficiency_cores = estimated_efficiency_core_count(values.len());
                let core_values = values
                    .iter()
                    .map(|(_, value)| *value as f64)
                    .collect::<Vec<_>>();

                to_cpu_average_sensor(&core_values)
                    .into_iter()
                    .chain(values.into_iter().map(|(index, value)| {
                        create_sensor(
                            &format!("TC{}C", index),
                            &to_cpu_core_label(index, efficiency_cores),
                            Some(value as f64),
                            "C",
                            "Cpu",
                            SensorSource::Smc,
                            None,
                        )
                    }))
                    .collect::<Vec<_>>()
            }
            Err(error) => {
                eprintln!("[mac-fan-ctrl] Failed reading CPU core temperatures: {error}");
                Vec::new()
            }
        }
    }

    fn read_gpu_temperature_sensors(&mut self) -> Vec<Sensor> {
        match self.smc.gpu_temperature() {
            Ok(gpu_temps) => {
                let clusters = [
                    maybe_temp_sensor("TG0D", "GPU Cluster 1", *gpu_temps.die, "Gpu"),
                    maybe_temp_sensor("TG0P", "GPU Cluster 2", *gpu_temps.proximity, "Gpu"),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();

                let values = clusters
                    .iter()
                    .filter_map(|sensor| sensor.value)
                    .collect::<Vec<_>>();
                let average = (!values.is_empty()).then(|| {
                    create_sensor(
                        "TGAVG",
                        "GPU Cluster Average",
                        Some(values.iter().sum::<f64>() / values.len() as f64),
                        "C",
                        "Gpu",
                        SensorSource::Derived,
                        None,
                    )
                });

                average.into_iter().chain(clusters).collect::<Vec<_>>()
            }
            Err(error) => {
                eprintln!("[mac-fan-ctrl] Failed reading GPU temperatures: {error}");
                Vec::new()
            }
        }
    }

    fn read_other_temperature_sensors(&mut self) -> Vec<Sensor> {
        match self.smc.other_temperatures() {
            Ok(other) => [
                maybe_temp_sensor("TM0P", "Memory Bank 1", *other.memory_bank_proximity, "Memory"),
                maybe_temp_sensor("TM1P", "Memory Bank 2", *other.mainboard_proximity, "Memory"),
                maybe_temp_sensor(
                    "TPCD",
                    "Power Manager Die Average",
                    *other.platform_controller_hub_die,
                    "Power",
                ),
                maybe_temp_sensor("TW0P", "Airport Proximity", *other.airport, "Other"),
                maybe_temp_sensor("TaLC", "Airflow Left", *other.airflow_left, "Other"),
                maybe_temp_sensor("TaRC", "Airflow Right", *other.airflow_right, "Other"),
                maybe_temp_sensor("TTLD", "Thunderbolt Left", *other.thunderbolt_left, "Other"),
                maybe_temp_sensor("TTRD", "Thunderbolt Right", *other.thunderbolt_right, "Other"),
                maybe_temp_sensor("Tm0P", "Mainboard", *other.mainboard_proximity, "Other"),
                maybe_temp_sensor("Th1H", "Heatpipe 1", *other.heatpipe_1, "Other"),
                maybe_temp_sensor("Th2H", "Heatpipe 2", *other.heatpipe_2, "Other"),
                maybe_temp_sensor("Ts0P", "Trackpad", *other.palm_rest_1, "Trackpad"),
                maybe_temp_sensor("Ts1P", "Trackpad Actuator", *other.palm_rest_2, "Trackpad"),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>(),
            Err(error) => {
                eprintln!("[mac-fan-ctrl] Failed reading other temperatures: {error}");
                Vec::new()
            }
        }
    }

    fn read_battery_sensors(&mut self) -> Vec<Sensor> {
        match self.smc.battery_info() {
            Ok(battery) => {
                let battery_main =
                    maybe_temp_sensor("TB0T", "Battery", *battery.temperature_max, "Battery");
                let gas_gauge_value = if *battery.temperature_1 > 0.0 {
                    *battery.temperature_1
                } else {
                    *battery.temperature_max
                };
                let gas_gauge =
                    maybe_temp_sensor("TB1T", "Battery Gas Gauge", gas_gauge_value, "Battery");

                [battery_main, gas_gauge]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
            }
            Err(error) => {
                eprintln!("[mac-fan-ctrl] Failed reading battery temperatures: {error}");
                Vec::new()
            }
        }
    }

    fn read_power_sensors(&mut self) -> Vec<Sensor> {
        let cpu_power_sensors = match self.smc.cpu_power() {
            Ok(cpu_power) => [
                maybe_power_sensor("PCPT", "CPU Total Power", *cpu_power.total),
                maybe_power_sensor("PCPC", "CPU Core Power", *cpu_power.core),
                maybe_power_sensor("PCPD", "CPU DRAM Power", *cpu_power.dram),
                maybe_power_sensor("PCPG", "CPU Graphics Power", *cpu_power.gfx),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>(),
            Err(error) => {
                eprintln!("[mac-fan-ctrl] Failed reading CPU power sensors: {error}");
                Vec::new()
            }
        };

        let gpu_power_sensor = self
            .smc
            .gpu_power()
            .ok()
            .and_then(|value| maybe_power_sensor("PG0R", "GPU Power", *value))
            .into_iter()
            .collect::<Vec<_>>();

        let dc_in_sensor = self
            .smc
            .power_dc_in()
            .ok()
            .and_then(|value| maybe_power_sensor("PDTR", "Power Supply Proximity", *value))
            .into_iter()
            .collect::<Vec<_>>();

        let system_total_sensor = self
            .smc
            .power_system_total()
            .ok()
            .and_then(|value| maybe_power_sensor("PSTR", "System Total Power", *value))
            .into_iter()
            .collect::<Vec<_>>();

        [cpu_power_sensors, gpu_power_sensor, dc_in_sensor, system_total_sensor]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
    }
}

fn data_value_to_f64(value: DataValue) -> Option<f64> {
    match value {
        DataValue::Float(value) => Some(value as f64),
        DataValue::Int(value) => Some(value as f64),
        DataValue::Uint(value) => Some(value as f64),
        _ => None,
    }
}

fn classify_dynamic_temperature_key(key: &str) -> (String, &'static str) {
    let normalized = key.to_uppercase();
    if normalized.starts_with("TG") || normalized.ends_with('G') {
        return (format!("Dynamic GPU {key}"), "Gpu");
    }
    if normalized.starts_with("TB") || normalized.starts_with("TG0") {
        return (format!("Dynamic Battery {key}"), "Battery");
    }
    if normalized.starts_with("TM") {
        return (format!("Dynamic Memory {key}"), "Memory");
    }
    if normalized.starts_with("TP") || normalized.starts_with("TC") {
        return (format!("Dynamic CPU {key}"), "Cpu");
    }
    (format!("Dynamic Sensor {key}"), "Other")
}

fn derive_apple_silicon_catalog_rows(dynamic: &[Sensor]) -> Vec<Sensor> {
    let normalized_rows = dynamic
        .iter()
        .map(|sensor| (sensor.key.to_uppercase(), sensor.value))
        .collect::<Vec<_>>();

    let mut cpu_die = dynamic
        .iter()
        .map(|sensor| (sensor.key.to_uppercase(), sensor.value))
        .filter(|(key, _)| key.starts_with("TP0") || (key.starts_with("TP") && key.ends_with('B')))
        .filter_map(|(key, value)| value.map(|value| (key, value)))
        .collect::<Vec<_>>();
    cpu_die.sort_by(|left, right| left.0.cmp(&right.0));

    let mut gpu_clusters = normalized_rows
        .iter()
        .filter(|(key, _)| key.starts_with("TG0") || (key.starts_with("TP") && key.ends_with('G')))
        .filter_map(|(key, value)| value.map(|value| (key.as_str(), value)))
        .collect::<Vec<_>>();
    gpu_clusters.sort_by(|left, right| left.0.cmp(right.0));

    let cpu_average = (!cpu_die.is_empty()).then(|| {
        let average = cpu_die.iter().map(|(_, value)| *value).sum::<f64>() / cpu_die.len() as f64;
        create_sensor(
            "TCPUAVG",
            "CPU Core Average",
            Some(average),
            "C",
            "Cpu",
            SensorSource::Derived,
            None,
        )
    });

    let cpu_cores = [
        ("TCE1", "CPU Efficiency Core 1"),
        ("TCE2", "CPU Efficiency Core 2"),
        ("TCP1", "CPU Performance Core 1"),
        ("TCP2", "CPU Performance Core 2"),
        ("TCP3", "CPU Performance Core 3"),
        ("TCP4", "CPU Performance Core 4"),
        ("TCP5", "CPU Performance Core 5"),
        ("TCP6", "CPU Performance Core 6"),
    ]
    .into_iter()
    .zip(cpu_die.into_iter())
    .map(|((key, name), (_, value))| {
        create_sensor(
            key,
            name,
            Some(value),
            "C",
            "Cpu",
            SensorSource::Derived,
            None,
        )
    })
    .collect::<Vec<_>>();

    let gpu_rows = [
        ("TG0D", "GPU Cluster 1"),
        ("TG0P", "GPU Cluster 2"),
        ("TGAVG", "GPU Cluster Average"),
    ]
    .into_iter()
    .enumerate()
    .filter_map(|(index, (key, name))| match key {
        "TGAVG" => {
            let values = gpu_clusters.iter().map(|(_, value)| *value).collect::<Vec<_>>();
            (!values.is_empty()).then(|| {
                create_sensor(
                    key,
                    name,
                    Some(values.iter().sum::<f64>() / values.len() as f64),
                    "C",
                    "Gpu",
                    SensorSource::Derived,
                    None,
                )
            })
        }
        _ => gpu_clusters.get(index).map(|(_, value)| {
            create_sensor(
                key,
                name,
                Some(*value),
                "C",
                "Gpu",
                SensorSource::Derived,
                None,
            )
        }),
    })
    .collect::<Vec<_>>();

    let power_die = cpu_cores
        .iter()
        .filter_map(|sensor| sensor.value)
        .collect::<Vec<_>>();
    let power_row = (!power_die.is_empty()).then(|| {
        create_sensor(
            "TPCD",
            "Power Manager Die Average",
            Some(power_die.iter().sum::<f64>() / power_die.len() as f64),
            "C",
            "Power",
            SensorSource::Derived,
            None,
        )
    });

    cpu_average
        .into_iter()
        .chain(cpu_cores)
        .chain(gpu_rows)
        .chain(power_row)
        .collect::<Vec<_>>()
}

fn detect_model_id() -> Option<String> {
    Command::new("sysctl")
        .args(["-n", "hw.model"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn diagnostics_enabled() -> bool {
    std::env::var("MAC_FAN_CTRL_SENSOR_DIAGNOSTICS")
        .ok()
        .is_some_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
}

fn should_use_apple_silicon_provider() -> bool {
    cfg!(target_arch = "aarch64")
        || std::env::var("MAC_FAN_CTRL_FORCE_APPLE_SILICON_PROVIDER")
            .ok()
            .is_some_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
}

fn merge_sensor_sets(primary: Vec<Sensor>, fallback: Vec<Sensor>) -> Vec<Sensor> {
    let mut merged = HashMap::<String, Sensor>::new();

    primary
        .into_iter()
        .chain(fallback)
        .for_each(|sensor| {
            let key = sensor.key.clone();
            merged
                .entry(key)
                .and_modify(|existing| {
                    if existing.value.is_none() && sensor.value.is_some() {
                        *existing = sensor.clone();
                    }
                })
                .or_insert(sensor);
        });

    merged.into_values().collect::<Vec<_>>()
}

impl SensorService {
    pub fn new() -> Self {
        Self {
            smc_client: SmcClient::new().ok(),
            diagnostics_enabled: diagnostics_enabled(),
            model_id: detect_model_id(),
            use_apple_silicon_provider: should_use_apple_silicon_provider(),
        }
    }

    fn read_smc_details(&mut self) -> Vec<Sensor> {
        if self.smc_client.is_none() {
            self.smc_client = SmcClient::new().ok();
        }

        self.smc_client
            .as_mut()
            .map(SmcClient::read_smc_details)
            .unwrap_or_default()
    }

    pub fn read_all_sensors(&mut self) -> Result<SensorData, SmcError> {
        let mut providers = vec!["smc".to_string()];
        let smc_details = self.read_smc_details();

        let (details, unresolved) = if self.use_apple_silicon_provider {
            let snapshot = read_apple_silicon_sensors();
            providers.insert(0, "iohid_iokit".to_string());
            (
                merge_sensor_sets(snapshot.sensors, smc_details),
                snapshot.unresolved,
            )
        } else {
            (smc_details, Vec::new())
        };

        let details = sort_sensors_for_display(ensure_reference_placeholders(details));
        let summary = build_summary(&details);
        let diagnostics = SensorDiagnostics {
            model_id: self.model_id.clone(),
            diagnostics_enabled: self.diagnostics_enabled,
            active_providers: providers,
            unresolved,
        };

        if self.diagnostics_enabled && !diagnostics.unresolved.is_empty() {
            eprintln!(
                "[mac-fan-ctrl] unresolved sensors: {}",
                diagnostics.unresolved.len()
            );
        }

        Ok(SensorData {
            summary,
            details,
            diagnostics,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_summary, derive_apple_silicon_catalog_rows, sort_sensors_for_display, NullReason,
        Sensor, SensorSource, SmcClient, SummarySensors,
    };

    fn sample_sensor(
        key: &str,
        name: &str,
        value: Option<f64>,
        unit: &str,
        sensor_type: &str,
    ) -> Sensor {
        Sensor {
            key: key.to_string(),
            name: name.to_string(),
            value,
            unit: unit.to_string(),
            sensor_type: sensor_type.to_string(),
            source: SensorSource::Smc,
            null_reason: None,
        }
    }

    #[test]
    fn build_summary_uses_preferred_sources() {
        let details = vec![
            sample_sensor("TGAVG", "GPU Cluster Average", Some(63.0), "C", "Gpu"),
            sample_sensor("TM0P", "Memory Bank 1", Some(51.0), "C", "Memory"),
            sample_sensor("TB0T", "Battery", Some(41.0), "C", "Battery"),
            sample_sensor("TCPUAVG", "CPU Core Average", Some(74.0), "C", "Cpu"),
        ];

        let summary = build_summary(&details);

        assert_eq!(summary.cpu_package.map(|sensor| sensor.key), Some("TCPUAVG".to_string()));
        assert_eq!(summary.gpu.map(|sensor| sensor.key), Some("TGAVG".to_string()));
        assert_eq!(summary.ram.map(|sensor| sensor.key), Some("TM0P".to_string()));
        assert_eq!(summary.ssd.map(|sensor| sensor.name), Some("APPLE SSD".to_string()));
    }

    #[test]
    fn sort_sensors_for_display_orders_by_type_then_name() {
        let details = vec![
            sample_sensor("PPTR", "Power Supply", Some(30.0), "W", "Power"),
            sample_sensor("TM0P", "Memory Bank 1", Some(52.0), "C", "Memory"),
            sample_sensor("TG0D", "GPU Die", Some(73.0), "C", "Gpu"),
            sample_sensor("TC0D", "CPU Die", Some(71.0), "C", "Cpu"),
        ];

        let ordered = sort_sensors_for_display(details);
        let ordered_types = ordered
            .iter()
            .map(|sensor| sensor.sensor_type.as_str())
            .collect::<Vec<_>>();

        assert_eq!(ordered_types, vec!["Cpu", "Gpu", "Memory", "Power"]);
    }

    #[test]
    fn build_summary_has_ssd_placeholder_when_storage_missing() {
        let details = vec![sample_sensor("TC0D", "CPU Die", Some(71.0), "C", "Cpu")];

        let summary = build_summary(&details);

        assert_eq!(
            summary,
            SummarySensors {
                cpu_package: Some(sample_sensor("TC0D", "CPU Die", Some(71.0), "C", "Cpu")),
                gpu: None,
                ram: None,
                ssd: Some(Sensor {
                    key: "SSD".to_string(),
                    name: "APPLE SSD".to_string(),
                    value: None,
                    unit: "C".to_string(),
                    sensor_type: "Storage".to_string(),
                    source: SensorSource::Placeholder,
                    null_reason: Some(NullReason::Placeholder),
                }),
            }
        );
    }

    #[test]
    fn derives_cpu_rows_from_pmu_die_candidates() {
        let dynamic = vec![
            sample_sensor("TP0b", "PMU tdie0", Some(57.0), "C", "Cpu"),
            sample_sensor("TP1b", "PMU tdie1", Some(58.0), "C", "Cpu"),
            sample_sensor("TP2b", "PMU tdie2", Some(63.0), "C", "Cpu"),
            sample_sensor("TP3b", "PMU tdie3", Some(64.0), "C", "Cpu"),
            sample_sensor("TP4b", "PMU tdie4", Some(65.0), "C", "Cpu"),
            sample_sensor("TP5b", "PMU tdie5", Some(66.0), "C", "Cpu"),
            sample_sensor("TP6b", "PMU tdie6", Some(67.0), "C", "Cpu"),
            sample_sensor("TP7b", "PMU tdie7", Some(68.0), "C", "Cpu"),
        ];

        let derived = derive_apple_silicon_catalog_rows(&dynamic);

        assert_eq!(
            derived
                .iter()
                .find(|sensor| sensor.key == "TCPUAVG")
                .and_then(|sensor| sensor.value),
            Some(63.5),
        );
        assert_eq!(
            derived
                .iter()
                .find(|sensor| sensor.key == "TCE1")
                .and_then(|sensor| sensor.value),
            Some(57.0),
        );
        assert_eq!(
            derived
                .iter()
                .find(|sensor| sensor.key == "TCP1")
                .and_then(|sensor| sensor.value),
            Some(63.0),
        );
    }

    #[test]
    fn derives_gpu_rows_from_pmu_graphics_candidates() {
        let dynamic = vec![
            sample_sensor("TP1g", "PMU TP1g", Some(52.0), "C", "Gpu"),
            sample_sensor("TP2g", "PMU TP2g", Some(54.0), "C", "Gpu"),
            sample_sensor("TP3g", "PMU TP3g", Some(56.0), "C", "Gpu"),
        ];

        let derived = derive_apple_silicon_catalog_rows(&dynamic);

        assert_eq!(
            derived
                .iter()
                .find(|sensor| sensor.key == "TG0D")
                .and_then(|sensor| sensor.value),
            Some(52.0),
        );
        assert_eq!(
            derived
                .iter()
                .find(|sensor| sensor.key == "TG0P")
                .and_then(|sensor| sensor.value),
            Some(54.0),
        );
        assert_eq!(
            derived
                .iter()
                .find(|sensor| sensor.key == "TGAVG")
                .and_then(|sensor| sensor.value),
            Some(54.0),
        );
    }

    #[test]
    fn derives_rows_from_lowercase_dynamic_keys() {
        let dynamic = vec![
            sample_sensor("Tp00", "Dynamic CPU Tp00", Some(57.0), "C", "Cpu"),
            sample_sensor("Tp01", "Dynamic CPU Tp01", Some(58.0), "C", "Cpu"),
            sample_sensor("Tp02", "Dynamic CPU Tp02", Some(59.0), "C", "Cpu"),
            sample_sensor("Tp03", "Dynamic CPU Tp03", Some(60.0), "C", "Cpu"),
            sample_sensor("Tp04", "Dynamic CPU Tp04", Some(61.0), "C", "Cpu"),
            sample_sensor("Tp05", "Dynamic CPU Tp05", Some(62.0), "C", "Cpu"),
            sample_sensor("Tp06", "Dynamic CPU Tp06", Some(63.0), "C", "Cpu"),
            sample_sensor("Tp07", "Dynamic CPU Tp07", Some(64.0), "C", "Cpu"),
            sample_sensor("Tg0C", "Dynamic GPU Tg0C", Some(55.0), "C", "Gpu"),
            sample_sensor("Tg0D", "Dynamic GPU Tg0D", Some(57.0), "C", "Gpu"),
        ];

        let derived = derive_apple_silicon_catalog_rows(&dynamic);
        let core_keys = derived
            .iter()
            .filter(|sensor| {
                sensor.key.starts_with("TCE")
                    || (sensor.key.starts_with("TCP") && sensor.key != "TCPUAVG")
            })
            .collect::<Vec<_>>();

        assert_eq!(core_keys.len(), 8);
        assert_eq!(
            derived
                .iter()
                .find(|sensor| sensor.key == "TG0D")
                .and_then(|sensor| sensor.value),
            Some(55.0),
        );
        assert_eq!(
            derived
                .iter()
                .find(|sensor| sensor.key == "TG0P")
                .and_then(|sensor| sensor.value),
            Some(57.0),
        );
    }

    #[test]
    #[ignore = "hardware-dependent smoke test for local Apple Silicon validation"]
    fn reads_dynamic_apple_silicon_values() {
        let mut client = SmcClient::new().expect("SMC client should connect on supported Macs");
        let details = client.read_smc_details();
        let has_cpu_average = details
            .iter()
            .any(|sensor| sensor.key == "TCPUAVG" && sensor.value.is_some());
        let has_gpu_cluster = details
            .iter()
            .any(|sensor| (sensor.key == "TG0D" || sensor.key == "TG0P") && sensor.value.is_some());

        assert!(
            has_cpu_average || has_gpu_cluster,
            "expected dynamic Apple Silicon mapping to provide CPU/GPU values"
        );
    }

}
