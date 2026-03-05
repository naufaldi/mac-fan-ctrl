use std::process::Command;

use crate::smc::{NullReason, Sensor, SensorSource, UnresolvedSensor};

#[derive(Debug, Default)]
pub struct AppleSiliconSnapshot {
    pub sensors: Vec<Sensor>,
    pub unresolved: Vec<UnresolvedSensor>,
}

fn parse_ioreg_dump(args: &[&str]) -> Option<String> {
    Command::new("ioreg")
        .args(args)
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
}

fn parse_number(value: &str) -> Option<f64> {
    value
        .trim()
        .trim_matches('"')
        .parse::<f64>()
        .ok()
}

fn to_celsius(raw: f64) -> f64 {
    if raw > 1000.0 {
        (raw / 10.0) - 273.15
    } else if raw > 200.0 {
        raw - 273.15
    } else {
        raw
    }
}

fn normalize_name(raw_name: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let lower = raw_name.to_lowercase();
    if lower.contains("nand") {
        return Some(("SSD", "APPLE SSD", "Storage"));
    }
    if lower.contains("gas gauge") {
        return Some(("TB1T", "Battery Gas Gauge", "Battery"));
    }
    if lower.contains("battery") {
        return Some(("TB0T", "Battery", "Battery"));
    }
    if lower.contains("pacc") {
        return Some(("TPCD", "Power Manager Die Average", "Power"));
    }
    if lower.contains("eacc") {
        return Some(("TCPUAVG", "CPU Core Average", "Cpu"));
    }
    if lower.contains("pmu tdie") {
        return Some(("TCPUAVG", "CPU Core Average", "Cpu"));
    }
    if lower.contains("pmu tp") {
        return Some(("TPCD", "Power Manager Die Average", "Power"));
    }
    if lower.contains("gpu") {
        return Some(("TGAVG", "GPU Cluster Average", "Gpu"));
    }
    None
}

pub fn read_apple_silicon_sensors() -> AppleSiliconSnapshot {
    let battery_dump = parse_ioreg_dump(&["-r", "-n", "AppleSmartBattery", "-l"]);
    let hid_dump = parse_ioreg_dump(&["-r", "-c", "IOHIDEventService", "-l"]);

    let battery_sensor = battery_dump.and_then(|dump| {
        dump.lines()
            .find(|line| line.contains("\"Temperature\""))
            .and_then(|line| line.split_once('='))
            .and_then(|(_, value)| parse_number(value))
            .map(to_celsius)
            .map(|value| Sensor {
                key: "TB0T".to_string(),
                name: "Battery".to_string(),
                value: Some(value),
                unit: "C".to_string(),
                sensor_type: "Battery".to_string(),
                source: SensorSource::IohidIokit,
                null_reason: None,
            })
    });

    let (hid_sensors, unresolved) = hid_dump
        .map(|dump| {
            let mut current_name = String::new();
            dump.lines().fold(
                (Vec::new(), Vec::new()),
                |(mut sensors, mut unresolved), line| {
                    if line.contains("\"Product\"") || line.contains("\"Name\"") {
                        current_name = line
                            .split_once('=')
                            .map(|(_, value)| value.trim().trim_matches('"').to_string())
                            .unwrap_or_default();

                        if let Some((key, name, sensor_type)) = normalize_name(&current_name) {
                            sensors.push(Sensor {
                                key: key.to_string(),
                                name: name.to_string(),
                                value: None,
                                unit: "C".to_string(),
                                sensor_type: sensor_type.to_string(),
                                source: SensorSource::IohidIokit,
                                null_reason: Some(NullReason::Unsupported),
                            });
                        }
                    }
                    if !line.contains("\"Temperature\"") {
                        return (sensors, unresolved);
                    }

                    let maybe_value = line
                        .split_once('=')
                        .and_then(|(_, value)| parse_number(value))
                        .map(to_celsius);

                    match (normalize_name(&current_name), maybe_value) {
                        (Some((key, name, sensor_type)), Some(value)) => {
                            sensors.push(Sensor {
                                key: key.to_string(),
                                name: name.to_string(),
                                value: Some(value),
                                unit: "C".to_string(),
                                sensor_type: sensor_type.to_string(),
                                source: SensorSource::IohidIokit,
                                null_reason: None,
                            });
                        }
                        (_, _) if current_name.is_empty() => {}
                        (None, Some(_)) => unresolved.push(UnresolvedSensor {
                            provider: "iohid_iokit".to_string(),
                            raw_name: current_name.clone(),
                            reason: "unmapped".to_string(),
                        }),
                        (None, None) => {}
                        (Some((key, name, sensor_type)), None) => sensors.push(Sensor {
                            key: key.to_string(),
                            name: name.to_string(),
                            value: None,
                            unit: "C".to_string(),
                            sensor_type: sensor_type.to_string(),
                            source: SensorSource::IohidIokit,
                            null_reason: Some(NullReason::ReadError),
                        }),
                    }

                    (sensors, unresolved)
                },
            )
        })
        .unwrap_or_default();

    let mut sensors = hid_sensors;
    if let Some(sensor) = battery_sensor {
        sensors.push(sensor);
    }

    AppleSiliconSnapshot { sensors, unresolved }
}
