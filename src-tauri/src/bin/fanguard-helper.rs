use std::ffi::CString;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use fanguard_lib::smc_protocol::{HelperRequest, HelperResponse, SOCKET_PATH};
use fanguard_lib::smc_writer::{SmcWriteApi, SmcWriter};

const HELPER_SOCKET_MODE: u32 = 0o660;
const ROOT_UID: libc::uid_t = 0;

fn main() {
    eprintln!("[fanguard-helper] Starting privileged helper daemon");

    let euid = unsafe { libc::geteuid() };
    if euid != 0 {
        eprintln!("[fanguard-helper] ERROR: Must run as root (current EUID: {euid})");
        std::process::exit(1);
    }

    let writer = match SmcWriter::new() {
        Ok(w) => {
            eprintln!("[fanguard-helper] SMC writer initialized successfully");
            w
        }
        Err(e) => {
            eprintln!("[fanguard-helper] FATAL: Failed to init SMC writer: {e}");
            std::process::exit(1);
        }
    };
    let writer = Arc::new(Mutex::new(writer));

    let _ = fs::remove_file(SOCKET_PATH);

    let listener = match UnixListener::bind(SOCKET_PATH) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[fanguard-helper] Failed to bind socket at {SOCKET_PATH}: {e}");
            std::process::exit(1);
        }
    };

    // Restrict socket to owner (root) and group (staff) only — no world access
    if let Err(e) = configure_socket_access(SOCKET_PATH) {
        eprintln!("[fanguard-helper] Failed to configure socket access: {e}");
        std::process::exit(1);
    }

    eprintln!("[fanguard-helper] Listening on {SOCKET_PATH}");

    let running = Arc::new(AtomicBool::new(true));
    let running_signal = running.clone();
    ctrlc::set_handler(move || {
        eprintln!("[fanguard-helper] Signal received - shutting down");
        running_signal.store(false, Ordering::SeqCst);
        let _ = fs::remove_file(SOCKET_PATH);
    })
    .unwrap_or_else(|e| eprintln!("[fanguard-helper] Signal handler error: {e}"));

    listener
        .set_nonblocking(true)
        .unwrap_or_else(|e| eprintln!("[fanguard-helper] Failed to set nonblocking: {e}"));

    while running.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _addr)) => {
                stream.set_nonblocking(false).unwrap_or_else(|e| {
                    eprintln!("[fanguard-helper] Failed to set blocking on client: {e}")
                });
                let writer_ref = Arc::clone(&writer);
                if let Err(e) = handle_client(stream, &writer_ref) {
                    eprintln!("[fanguard-helper] Client error: {e}");
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(e) => {
                eprintln!("[fanguard-helper] Accept error: {e}");
            }
        }
    }

    let _ = fs::remove_file(SOCKET_PATH);
    eprintln!("[fanguard-helper] Shutdown complete");
}

fn handle_client(stream: UnixStream, writer: &Mutex<SmcWriter>) -> std::io::Result<()> {
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap_or(());
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

fn helper_socket_group_name() -> &'static str {
    "staff"
}

fn configure_socket_access(path: &str) -> std::io::Result<()> {
    let staff_gid = resolve_group_id(helper_socket_group_name())?;
    chown_socket(path, ROOT_UID, staff_gid)?;
    fs::set_permissions(path, fs::Permissions::from_mode(HELPER_SOCKET_MODE))
}

fn resolve_group_id(group_name: &str) -> std::io::Result<libc::gid_t> {
    let c_group_name = CString::new(group_name)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    let group = unsafe { libc::getgrnam(c_group_name.as_ptr()) };
    if group.is_null() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("group '{group_name}' not found"),
        ));
    }

    Ok(unsafe { (*group).gr_gid })
}

fn chown_socket(path: &str, uid: libc::uid_t, gid: libc::gid_t) -> std::io::Result<()> {
    let c_path =
        CString::new(path).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    let result = unsafe { libc::chown(c_path.as_ptr(), uid, gid) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
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

#[cfg(test)]
mod tests {
    use super::helper_socket_group_name;

    #[test]
    fn helper_socket_uses_staff_group_for_gui_user_access() {
        assert_eq!(helper_socket_group_name(), "staff");
    }
}
