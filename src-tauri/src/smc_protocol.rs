use serde::{Deserialize, Serialize};

pub const SOCKET_PATH: &str = "/var/run/fanguard.sock";
pub const HELPER_PROTOCOL_VERSION: u32 = 2;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum HelperRequest {
    GetProtocolVersion,
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
    ProtocolVersion { version: u32 },
    Ok,
    OkDiagnose { lines: Vec<String> },
    Pong,
    Error { message: String },
}
