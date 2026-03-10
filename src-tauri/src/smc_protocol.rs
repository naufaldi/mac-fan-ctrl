use serde::{Deserialize, Serialize};

pub const SOCKET_PATH: &str = "/var/run/fanguard.sock";

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum HelperRequest {
    SetFanTargetRpm {
        fan_index: u8,
        rpm: f32,
        min_rpm: f32,
        max_rpm: f32,
    },
    SetFanAuto {
        fan_index: u8,
    },
    LockFanControl,
    UnlockFanControl,
    DiagnoseFanControl,
    Ping,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum HelperResponse {
    Ok,
    OkDiagnose { lines: Vec<String> },
    Pong,
    Error { message: String },
}
