use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use mac_fan_ctrl_lib::smc_protocol::{HelperRequest, HelperResponse, SOCKET_PATH};
use mac_fan_ctrl_lib::smc_writer::{SmcWriteApi, SmcWriter};

fn main() {
    eprintln!("[mac-fan-ctrl-helper] Starting privileged helper daemon");

    let euid = unsafe { libc::geteuid() };
    if euid != 0 {
        eprintln!("[mac-fan-ctrl-helper] ERROR: Must run as root (current EUID: {euid})");
        std::process::exit(1);
    }

    let writer = match SmcWriter::new() {
        Ok(w) => {
            eprintln!("[mac-fan-ctrl-helper] SMC writer initialized successfully");
            w
        }
        Err(e) => {
            eprintln!("[mac-fan-ctrl-helper] FATAL: Failed to init SMC writer: {e}");
            std::process::exit(1);
        }
    };
    let writer = Arc::new(Mutex::new(writer));

    let _ = fs::remove_file(SOCKET_PATH);

    let listener = match UnixListener::bind(SOCKET_PATH) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[mac-fan-ctrl-helper] Failed to bind socket at {SOCKET_PATH}: {e}");
            std::process::exit(1);
        }
    };

    // Restrict socket to owner (root) and group (staff) only — no world access
    if let Err(e) = fs::set_permissions(SOCKET_PATH, fs::Permissions::from_mode(0o660)) {
        eprintln!("[mac-fan-ctrl-helper] Failed to set socket permissions: {e}");
    }

    eprintln!("[mac-fan-ctrl-helper] Listening on {SOCKET_PATH}");

    let running = Arc::new(AtomicBool::new(true));
    let running_signal = running.clone();
    ctrlc::set_handler(move || {
        eprintln!("[mac-fan-ctrl-helper] Signal received - shutting down");
        running_signal.store(false, Ordering::SeqCst);
        let _ = fs::remove_file(SOCKET_PATH);
    })
    .unwrap_or_else(|e| eprintln!("[mac-fan-ctrl-helper] Signal handler error: {e}"));

    listener
        .set_nonblocking(true)
        .unwrap_or_else(|e| eprintln!("[mac-fan-ctrl-helper] Failed to set nonblocking: {e}"));

    while running.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _addr)) => {
                stream
                    .set_nonblocking(false)
                    .unwrap_or_else(|e| {
                        eprintln!("[mac-fan-ctrl-helper] Failed to set blocking on client: {e}")
                    });
                let writer_ref = Arc::clone(&writer);
                if let Err(e) = handle_client(stream, &writer_ref) {
                    eprintln!("[mac-fan-ctrl-helper] Client error: {e}");
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(e) => {
                eprintln!("[mac-fan-ctrl-helper] Accept error: {e}");
            }
        }
    }

    let _ = fs::remove_file(SOCKET_PATH);
    eprintln!("[mac-fan-ctrl-helper] Shutdown complete");
}

fn handle_client(stream: UnixStream, writer: &Mutex<SmcWriter>) -> std::io::Result<()> {
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap_or(());
    let limited = (&stream).take(8192);
    let mut reader = BufReader::new(limited);
    let mut line = String::new();
    reader.read_line(&mut line)?;

    if line.trim().is_empty() {
        return Ok(());
    }

    let request: HelperRequest = match serde_json::from_str(&line) {
        Ok(r) => r,
        Err(e) => {
            let response = HelperResponse::Error {
                message: format!("Invalid request: {e}"),
            };
            return send_response(&stream, &response);
        }
    };

    let response = dispatch_request(request, writer);
    send_response(&stream, &response)
}

fn send_response(stream: &UnixStream, response: &HelperResponse) -> std::io::Result<()> {
    let mut writer = stream.try_clone()?;
    let mut resp_line = serde_json::to_string(response)
        .unwrap_or_else(|_| r#"{"status":"error","message":"serialization failed"}"#.to_string());
    resp_line.push('\n');
    writer.write_all(resp_line.as_bytes())
}

fn dispatch_request(request: HelperRequest, writer: &Mutex<SmcWriter>) -> HelperResponse {
    let guard = match writer.lock() {
        Ok(g) => g,
        Err(e) => {
            return HelperResponse::Error {
                message: format!("Writer lock poisoned: {e}"),
            }
        }
    };

    match request {
        HelperRequest::Ping => HelperResponse::Pong,
        HelperRequest::SetFanTargetRpm {
            fan_index,
            rpm,
            min_rpm,
            max_rpm,
        } => match guard.set_fan_target_rpm(fan_index, rpm, min_rpm, max_rpm) {
            Ok(()) => HelperResponse::Ok,
            Err(e) => HelperResponse::Error {
                message: e.to_string(),
            },
        },
        HelperRequest::SetFanAuto { fan_index } => match guard.set_fan_auto(fan_index) {
            Ok(()) => HelperResponse::Ok,
            Err(e) => HelperResponse::Error {
                message: e.to_string(),
            },
        },
        HelperRequest::LockFanControl => match guard.lock_fan_control() {
            Ok(()) => HelperResponse::Ok,
            Err(e) => HelperResponse::Error {
                message: e.to_string(),
            },
        },
        HelperRequest::UnlockFanControl => match guard.unlock_fan_control() {
            Ok(()) => HelperResponse::Ok,
            Err(e) => HelperResponse::Error {
                message: e.to_string(),
            },
        },
        HelperRequest::DiagnoseFanControl => HelperResponse::OkDiagnose {
            lines: guard.diagnose_fan_control(),
        },
    }
}
