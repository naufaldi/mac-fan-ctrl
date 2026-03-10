use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use crate::smc_protocol::{HelperRequest, HelperResponse, SOCKET_PATH};
use crate::smc_writer::{SmcWriteApi, SmcWriteError};

pub struct SmcSocketClient {
    socket_path: PathBuf,
}

impl SmcSocketClient {
    pub fn new() -> Result<Self, SmcWriteError> {
        Self::with_path(SOCKET_PATH)
    }

    pub fn with_path(path: &str) -> Result<Self, SmcWriteError> {
        let socket_path = PathBuf::from(path);
        let stream = UnixStream::connect(&socket_path)
            .map_err(|_| SmcWriteError::HelperNotRunning)?;
        stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .map_err(|e| SmcWriteError::HelperError(e.to_string()))?;
        stream.set_write_timeout(Some(std::time::Duration::from_secs(5)))
            .map_err(|e| SmcWriteError::HelperError(e.to_string()))?;
        let response = send_request_on(&stream, &HelperRequest::Ping)?;
        match response {
            HelperResponse::Pong => Ok(Self { socket_path }),
            HelperResponse::Error { message } => {
                Err(SmcWriteError::HelperError(message))
            }
            _ => Err(SmcWriteError::HelperError("unexpected ping response".to_string())),
        }
    }

    fn send_request(&self, request: &HelperRequest) -> Result<HelperResponse, SmcWriteError> {
        let stream = UnixStream::connect(&self.socket_path)
            .map_err(|_| SmcWriteError::HelperNotRunning)?;
        send_request_on(&stream, request)
    }
}

fn send_request_on(
    stream: &UnixStream,
    request: &HelperRequest,
) -> Result<HelperResponse, SmcWriteError> {
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .map_err(|e| SmcWriteError::HelperError(e.to_string()))?;
    stream.set_write_timeout(Some(std::time::Duration::from_secs(5)))
        .map_err(|e| SmcWriteError::HelperError(e.to_string()))?;
    let mut writer = stream.try_clone()
        .map_err(|e| SmcWriteError::HelperError(e.to_string()))?;
    let mut line = serde_json::to_string(request)
        .map_err(|e| SmcWriteError::HelperError(e.to_string()))?;
    line.push('\n');
    writer.write_all(line.as_bytes())
        .map_err(|e| SmcWriteError::HelperError(e.to_string()))?;

    let mut reader = BufReader::new(stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line)
        .map_err(|e| SmcWriteError::HelperError(e.to_string()))?;
    serde_json::from_str(&response_line)
        .map_err(|e| SmcWriteError::HelperError(e.to_string()))
}

fn response_to_result(response: HelperResponse) -> Result<(), SmcWriteError> {
    match response {
        HelperResponse::Ok => Ok(()),
        HelperResponse::Error { message } => Err(SmcWriteError::HelperError(message)),
        _ => Err(SmcWriteError::HelperError("unexpected response type".to_string())),
    }
}

impl SmcWriteApi for SmcSocketClient {
    fn set_fan_target_rpm(
        &self,
        fan_index: u8,
        rpm: f32,
        min_rpm: f32,
        max_rpm: f32,
    ) -> Result<(), SmcWriteError> {
        let response = self.send_request(&HelperRequest::SetFanTargetRpm {
            fan_index,
            rpm,
            min_rpm,
            max_rpm,
        })?;
        response_to_result(response)
    }

    fn set_fan_auto(&self, fan_index: u8) -> Result<(), SmcWriteError> {
        let response = self.send_request(&HelperRequest::SetFanAuto { fan_index })?;
        response_to_result(response)
    }

    fn lock_fan_control(&self) -> Result<(), SmcWriteError> {
        let response = self.send_request(&HelperRequest::LockFanControl)?;
        response_to_result(response)
    }

    fn unlock_fan_control(&self) -> Result<(), SmcWriteError> {
        let response = self.send_request(&HelperRequest::UnlockFanControl)?;
        response_to_result(response)
    }

    fn diagnose_fan_control(&self) -> Vec<String> {
        match self.send_request(&HelperRequest::DiagnoseFanControl) {
            Ok(HelperResponse::OkDiagnose { lines }) => lines,
            Ok(HelperResponse::Error { message }) => vec![format!("Helper error: {message}")],
            Ok(_) => vec!["Unexpected response from helper".to_string()],
            Err(e) => vec![format!("Socket error: {e}")],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;

    fn with_mock_helper<F>(handler: F, test: impl FnOnce(&str))
    where
        F: Fn(HelperRequest) -> HelperResponse + Send + 'static,
    {
        let dir = tempfile::tempdir().unwrap();
        let sock_path = dir.path().join("test.sock");
        let sock_path_str = sock_path.to_str().unwrap().to_string();

        let listener = UnixListener::bind(&sock_path).unwrap();
        let handle = std::thread::spawn(move || {
            for stream in listener.incoming().take(5) {
                let stream = match stream { Ok(s) => s, Err(_) => break };
                let mut reader = BufReader::new(&stream);
                let mut line = String::new();
                if reader.read_line(&mut line).is_err() { break; }
                if line.is_empty() { continue; }
                let request: HelperRequest = match serde_json::from_str(&line) {
                    Ok(r) => r,
                    Err(_) => continue,
                };
                let response = handler(request);
                let mut resp_line = serde_json::to_string(&response).unwrap();
                resp_line.push('\n');
                let mut writer = stream.try_clone().unwrap();
                let _ = writer.write_all(resp_line.as_bytes());
            }
        });

        test(&sock_path_str);
        drop(handle);
    }

    #[test]
    fn connect_and_ping() {
        with_mock_helper(
            |req| match req {
                HelperRequest::Ping => HelperResponse::Pong,
                _ => HelperResponse::Error { message: "unexpected".into() },
            },
            |path| {
                let client = SmcSocketClient::with_path(path);
                assert!(client.is_ok());
            },
        );
    }

    #[test]
    fn set_fan_auto_sends_correct_request() {
        with_mock_helper(
            |req| match req {
                HelperRequest::Ping => HelperResponse::Pong,
                HelperRequest::SetFanAuto { fan_index: 0 } => HelperResponse::Ok,
                _ => HelperResponse::Error { message: "unexpected".into() },
            },
            |path| {
                let client = SmcSocketClient::with_path(path).unwrap();
                let result = client.set_fan_auto(0);
                assert!(result.is_ok());
            },
        );
    }

    #[test]
    fn error_response_maps_to_smc_write_error() {
        with_mock_helper(
            |req| match req {
                HelperRequest::Ping => HelperResponse::Pong,
                HelperRequest::SetFanAuto { .. } => HelperResponse::Error {
                    message: "Insufficient privileges".into(),
                },
                _ => HelperResponse::Error { message: "unexpected".into() },
            },
            |path| {
                let client = SmcSocketClient::with_path(path).unwrap();
                let result = client.set_fan_auto(0);
                assert!(result.is_err());
                assert!(result.unwrap_err().to_string().contains("Insufficient privileges"));
            },
        );
    }

    #[test]
    fn connect_fails_when_no_socket() {
        let result = SmcSocketClient::with_path("/tmp/nonexistent-mac-fan-ctrl-test.sock");
        assert!(result.is_err());
    }
}
