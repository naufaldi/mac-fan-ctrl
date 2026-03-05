use macsmc::Smc;
use serde::Serialize;
use thiserror::Error;

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
}

pub struct SmcClient {
    smc: Smc,
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

fn create_sensor(key: &str, name: &str, value: Option<f64>, unit: &str, sensor_type: &str) -> Sensor {
    Sensor {
        key: key.to_string(),
        name: name.to_string(),
        value,
        unit: unit.to_string(),
        sensor_type: sensor_type.to_string(),
    }
}

fn maybe_temp_sensor(key: &str, name: &str, value: f32, sensor_type: &str) -> Option<Sensor> {
    (value > 0.0).then(|| create_sensor(key, name, Some(value as f64), "C", sensor_type))
}

fn maybe_power_sensor(key: &str, name: &str, value: f32) -> Option<Sensor> {
    (value > 0.0).then(|| create_sensor(key, name, Some(value as f64), "W", "Power"))
}

fn has_sensor_type(details: &[Sensor], sensor_type: &str) -> bool {
    details.iter().any(|sensor| sensor.sensor_type == sensor_type)
}

fn has_sensor_named(details: &[Sensor], name: &str) -> bool {
    details.iter().any(|sensor| sensor.name == name)
}

fn ensure_reference_placeholders(details: Vec<Sensor>) -> Vec<Sensor> {
    let placeholder_specs = [
        ("SSD", "SSD", "C", "Storage"),
        ("TRKP", "Trackpad", "C", "Trackpad"),
        ("TM0P", "Memory Bank 1", "C", "Memory"),
        ("TB0T", "Battery", "C", "Battery"),
        ("PDTR", "Power Supply", "W", "Power"),
    ];

    let placeholders = placeholder_specs
        .into_iter()
        .filter(|(key, name, _unit, sensor_type)| {
            !has_sensor_type(&details, sensor_type) && !has_sensor_named(&details, name) && !details.iter().any(|sensor| sensor.key == *key)
        })
        .map(|(key, name, unit, sensor_type)| create_sensor(key, name, None, unit, sensor_type))
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
    let cpu_package = pick_summary_sensor(details, &["TC0D", "TC0P"], "Cpu");
    let gpu = pick_summary_sensor(details, &["TG0D", "TG0P"], "Gpu");
    let ram = pick_summary_sensor(details, &["TM0P"], "Memory");
    let ssd = pick_summary_sensor(details, &["TH0P", "TH1P", "SSD"], "Storage")
        .or_else(|| Some(create_sensor("SSD", "SSD", None, "C", "Storage")));

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

    pub fn read_all_sensors(&mut self) -> Result<SensorData, SmcError> {
        let details = [
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

        let details = sort_sensors_for_display(ensure_reference_placeholders(details));
        let summary = build_summary(&details);

        Ok(SensorData { summary, details })
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
            Ok(core_temps) => core_temps
                .enumerate()
                .filter_map(|(index, celsius)| {
                    celsius.ok().and_then(|value| {
                        maybe_temp_sensor(
                            &format!("TC{}C", index),
                            &format!("CPU Core {}", index + 1),
                            *value,
                            "Cpu",
                        )
                    })
                })
                .collect::<Vec<_>>(),
            Err(error) => {
                eprintln!("[mac-fan-ctrl] Failed reading CPU core temperatures: {error}");
                Vec::new()
            }
        }
    }

    fn read_gpu_temperature_sensors(&mut self) -> Vec<Sensor> {
        match self.smc.gpu_temperature() {
            Ok(gpu_temps) => [
                maybe_temp_sensor("TG0D", "GPU Die", *gpu_temps.die, "Gpu"),
                maybe_temp_sensor("TG0P", "GPU Proximity", *gpu_temps.proximity, "Gpu"),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>(),
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
                maybe_temp_sensor(
                    "TPCD",
                    "Platform Controller Hub",
                    *other.platform_controller_hub_die,
                    "Other",
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
            Ok(battery) => [
                maybe_temp_sensor("TB0T", "Battery", *battery.temperature_max, "Battery"),
                maybe_temp_sensor("TB1T", "Battery Sensor 1", *battery.temperature_1, "Battery"),
                maybe_temp_sensor("TB2T", "Battery Sensor 2", *battery.temperature_2, "Battery"),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>(),
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
            .and_then(|value| maybe_power_sensor("PDTR", "Power Supply", *value))
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

#[cfg(test)]
mod tests {
    use super::{build_summary, sort_sensors_for_display, Sensor, SummarySensors};

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
        }
    }

    #[test]
    fn build_summary_uses_preferred_sources() {
        let details = vec![
            sample_sensor("TG0P", "GPU Proximity", Some(63.0), "C", "Gpu"),
            sample_sensor("TM0P", "Memory Bank 1", Some(51.0), "C", "Memory"),
            sample_sensor("TB0T", "Battery", Some(41.0), "C", "Battery"),
            sample_sensor("TC0D", "CPU Die", Some(74.0), "C", "Cpu"),
        ];

        let summary = build_summary(&details);

        assert_eq!(summary.cpu_package.map(|sensor| sensor.key), Some("TC0D".to_string()));
        assert_eq!(summary.gpu.map(|sensor| sensor.key), Some("TG0P".to_string()));
        assert_eq!(summary.ram.map(|sensor| sensor.key), Some("TM0P".to_string()));
        assert_eq!(summary.ssd.map(|sensor| sensor.name), Some("SSD".to_string()));
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
                ssd: Some(sample_sensor("SSD", "SSD", None, "C", "Storage")),
            }
        );
    }
}
