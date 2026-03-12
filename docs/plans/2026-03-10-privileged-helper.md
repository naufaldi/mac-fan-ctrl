# Privileged Helper Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Split SMC write operations into a privileged helper daemon so the main app runs as the normal user, fixing window management bugs (#40, #42, #44).

**Architecture:** A Unix domain socket helper binary (`mac-fan-ctrl-helper`) runs as root and handles all SMC writes. The main app connects via `SmcSocketClient` which implements the existing `SmcWriteApi` trait — zero changes needed in command handlers. The protocol is newline-delimited JSON over `/var/run/mac-fan-ctrl.sock`.

**Tech Stack:** Rust (serde_json for protocol), Unix domain sockets (`std::os::unix::net`), existing `SmcWriteApi` trait, LaunchDaemon for persistence.

---

### Task 1: Define Shared Socket Protocol Types

**Files:**
- Create: `src-tauri/src/smc_protocol.rs`
- Modify: `src-tauri/src/main.rs:1-11` (add `mod smc_protocol;`)

**Step 1: Write the protocol types**

Create `src-tauri/src/smc_protocol.rs` with request/response enums:

```rust
use serde::{Deserialize, Serialize};

pub const SOCKET_PATH: &str = "/var/run/mac-fan-ctrl.sock";

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
```

**Step 2: Add module declaration**

In `src-tauri/src/main.rs`, add `mod smc_protocol;` after `mod smc_writer;` (line 10).

**Step 3: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: Compiles with no errors.

**Step 4: Commit**

```bash
git add src-tauri/src/smc_protocol.rs src-tauri/src/main.rs
git commit -m "feat: add shared socket protocol types for privileged helper"
```

---

### Task 2: Create SmcSocketClient (with tests)

**Files:**
- Create: `src-tauri/src/smc_client.rs`
- Modify: `src-tauri/src/main.rs` (add `mod smc_client;`)

**Step 1: Write failing tests for SmcSocketClient**

Create `src-tauri/src/smc_client.rs` with the test module first:

```rust
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

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
        // Verify socket exists and is connectable
        let stream = UnixStream::connect(&socket_path)
            .map_err(|_| SmcWriteError::HelperNotRunning)?;
        // Send a ping to verify the helper is responsive
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
            // Accept up to 5 connections for the test
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
```

**Step 2: Add new SmcWriteError variants**

In `src-tauri/src/smc_writer.rs`, add two variants to `SmcWriteError` (after `ModeVerificationFailed`):

```rust
    #[error("Privileged helper is not running")]
    HelperNotRunning,
    #[error("Helper communication error: {0}")]
    HelperError(String),
```

**Step 3: Add module declaration**

In `src-tauri/src/main.rs`, add `mod smc_client;` after `mod smc_protocol;`.

**Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test smc_client`
Expected: All 4 tests pass.

**Step 5: Commit**

```bash
git add src-tauri/src/smc_client.rs src-tauri/src/smc_writer.rs src-tauri/src/main.rs
git commit -m "feat: add SmcSocketClient implementing SmcWriteApi over Unix socket"
```

---

### Task 3: Create Helper Binary

**Files:**
- Create: `src-tauri/src/bin/mac-fan-ctrl-helper.rs`
- Modify: `src-tauri/Cargo.toml` (add `[[bin]]` section)

**Step 1: Add binary target to Cargo.toml**

Add after `[profile.release]` section:

```toml
[[bin]]
name = "mac-fan-ctrl-helper"
path = "src/bin/mac-fan-ctrl-helper.rs"
```

**Step 2: Create the helper binary**

Create `src-tauri/src/bin/mac-fan-ctrl-helper.rs`:

```rust
//! Privileged helper daemon for mac-fan-ctrl.
//! Runs as root, listens on a Unix domain socket, handles SMC write commands.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// We need to reference the main crate's modules.
// Since this is a binary in the same crate, we use `mac_fan_ctrl::` paths.
// However, since modules are private, we duplicate the minimal types needed.

mod helper_protocol {
    use serde::{Deserialize, Serialize};

    pub const SOCKET_PATH: &str = "/var/run/mac-fan-ctrl.sock";

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
}

use helper_protocol::{HelperRequest, HelperResponse, SOCKET_PATH};

// ── Minimal SMC writer FFI (duplicated from smc_writer.rs) ──────────────────
// We duplicate only the IOKit FFI and SmcWriter here because the main crate's
// modules are private. A future refactor could extract a shared library crate.
// For now, this keeps the helper self-contained and avoids Tauri dependencies.

// NOTE: The helper binary includes smc_writer.rs logic via a build-time include
// or by extracting the core SMC FFI into a shared module. For the initial
// implementation, we use the approach below.

fn main() {
    eprintln!("[mac-fan-ctrl-helper] Starting privileged helper daemon");

    // Verify we're running as root
    let euid = unsafe { libc::geteuid() };
    if euid != 0 {
        eprintln!("[mac-fan-ctrl-helper] ERROR: Must run as root (current EUID: {euid})");
        std::process::exit(1);
    }

    // Clean up stale socket
    let _ = fs::remove_file(SOCKET_PATH);

    // Bind socket
    let listener = match UnixListener::bind(SOCKET_PATH) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[mac-fan-ctrl-helper] Failed to bind socket at {SOCKET_PATH}: {e}");
            std::process::exit(1);
        }
    };

    // Set socket permissions: owner rw, group rw, others rw (anyone can connect)
    // Security is handled by validating commands, not restricting connections.
    if let Err(e) = fs::set_permissions(SOCKET_PATH, fs::Permissions::from_mode(0o666)) {
        eprintln!("[mac-fan-ctrl-helper] Failed to set socket permissions: {e}");
    }

    eprintln!("[mac-fan-ctrl-helper] Listening on {SOCKET_PATH}");

    // Setup signal handler for graceful shutdown
    let running = Arc::new(AtomicBool::new(true));
    let running_signal = running.clone();
    ctrlc::set_handler(move || {
        eprintln!("[mac-fan-ctrl-helper] Signal received — shutting down");
        running_signal.store(false, Ordering::SeqCst);
        // Remove socket file on shutdown
        let _ = fs::remove_file(SOCKET_PATH);
    })
    .unwrap_or_else(|e| eprintln!("[mac-fan-ctrl-helper] Signal handler error: {e}"));

    // Set non-blocking so we can check the running flag
    listener.set_nonblocking(true).unwrap_or(());

    while running.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _addr)) => {
                // Set blocking mode for the client connection
                stream.set_nonblocking(false).unwrap_or(());
                if let Err(e) = handle_client(stream) {
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

    // Cleanup
    let _ = fs::remove_file(SOCKET_PATH);
    eprintln!("[mac-fan-ctrl-helper] Shutdown complete");
}

fn handle_client(stream: UnixStream) -> std::io::Result<()> {
    let mut reader = BufReader::new(&stream);
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
            send_response(&stream, &response)?;
            return Ok(());
        }
    };

    let response = dispatch_request(request);
    send_response(&stream, &response)
}

fn send_response(stream: &UnixStream, response: &HelperResponse) -> std::io::Result<()> {
    let mut writer = stream.try_clone()?;
    let mut resp_line = serde_json::to_string(response)
        .unwrap_or_else(|_| r#"{"status":"error","message":"serialization failed"}"#.to_string());
    resp_line.push('\n');
    writer.write_all(resp_line.as_bytes())
}

fn dispatch_request(request: HelperRequest) -> HelperResponse {
    // TODO: Task 4 will integrate actual SmcWriter here.
    // For now, return a stub response so the binary compiles and the protocol works.
    match request {
        HelperRequest::Ping => HelperResponse::Pong,
        HelperRequest::DiagnoseFanControl => HelperResponse::OkDiagnose {
            lines: vec!["Helper running (stub mode)".to_string()],
        },
        _ => HelperResponse::Error {
            message: "SMC writer not yet integrated — stub mode".to_string(),
        },
    }
}
```

**Step 3: Verify it compiles**

Run: `cd src-tauri && cargo build --bin mac-fan-ctrl-helper`
Expected: Compiles successfully.

**Step 4: Commit**

```bash
git add src-tauri/src/bin/mac-fan-ctrl-helper.rs src-tauri/Cargo.toml
git commit -m "feat: add privileged helper binary with socket listener (stub mode)"
```

---

### Task 4: Extract SMC Writer Core Into Helper

**Files:**
- Modify: `src-tauri/src/bin/mac-fan-ctrl-helper.rs`

This task integrates the real `SmcWriter` into the helper. Since `smc_writer.rs` is a private module in the main binary crate, the cleanest approach is to make the core SMC FFI code available to the helper binary.

**Step 1: Restructure the crate to expose smc_writer as a library**

Modify `src-tauri/Cargo.toml` — add a `[lib]` section before `[[bin]]`:

```toml
[lib]
name = "mac_fan_ctrl_lib"
path = "src/lib.rs"
```

**Step 2: Create `src-tauri/src/lib.rs`**

```rust
// Re-export modules needed by the helper binary
pub mod smc_writer;
pub mod smc_protocol;
pub mod log;
```

**Step 3: Update main.rs module visibility**

The modules now declared in `lib.rs` should be used via the library crate in `main.rs`. Update `main.rs` to replace:
```rust
mod smc_writer;
mod smc_protocol;
```
with:
```rust
use mac_fan_ctrl_lib::smc_writer;
use mac_fan_ctrl_lib::smc_protocol;
```

Keep other private modules as `mod` declarations since they depend on Tauri.

Note: The `log` module has macros — it needs `#[macro_use]` or `pub use` handling. If the log macros cause issues, the simplest approach is to duplicate the 2-line eprintln macro in the helper binary instead.

**Step 4: Update the helper to use real SmcWriter**

Replace the `dispatch_request` stub in `mac-fan-ctrl-helper.rs`:

```rust
use mac_fan_ctrl_lib::smc_writer::{SmcWriteApi, SmcWriter};
use std::sync::Mutex;

// In main(), after root check:
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
let writer = Mutex::new(writer);

// Pass writer to handle_client and dispatch_request via closure or global.
// Simplest: use a lazy_static or pass as &Mutex<SmcWriter>.
```

Update `dispatch_request` to use the real writer:

```rust
fn dispatch_request(
    request: HelperRequest,
    writer: &Mutex<SmcWriter>,
) -> HelperResponse {
    let guard = match writer.lock() {
        Ok(g) => g,
        Err(e) => return HelperResponse::Error {
            message: format!("Writer lock poisoned: {e}"),
        },
    };

    match request {
        HelperRequest::Ping => HelperResponse::Pong,
        HelperRequest::SetFanTargetRpm { fan_index, rpm, min_rpm, max_rpm } => {
            match guard.set_fan_target_rpm(fan_index, rpm, min_rpm, max_rpm) {
                Ok(()) => HelperResponse::Ok,
                Err(e) => HelperResponse::Error { message: e.to_string() },
            }
        }
        HelperRequest::SetFanAuto { fan_index } => {
            match guard.set_fan_auto(fan_index) {
                Ok(()) => HelperResponse::Ok,
                Err(e) => HelperResponse::Error { message: e.to_string() },
            }
        }
        HelperRequest::LockFanControl => {
            match guard.lock_fan_control() {
                Ok(()) => HelperResponse::Ok,
                Err(e) => HelperResponse::Error { message: e.to_string() },
            }
        }
        HelperRequest::UnlockFanControl => {
            match guard.unlock_fan_control() {
                Ok(()) => HelperResponse::Ok,
                Err(e) => HelperResponse::Error { message: e.to_string() },
            }
        }
        HelperRequest::DiagnoseFanControl => {
            HelperResponse::OkDiagnose { lines: guard.diagnose_fan_control() }
        }
    }
}
```

**Step 5: Verify it compiles**

Run: `cd src-tauri && cargo build --bin mac-fan-ctrl-helper`
Expected: Compiles. May need to adjust log macro imports.

**Step 6: Integration test — run helper manually**

```bash
cd src-tauri && sudo cargo run --bin mac-fan-ctrl-helper
# In another terminal:
echo '{"cmd":"ping"}' | socat - UNIX-CONNECT:/var/run/mac-fan-ctrl.sock
# Expected: {"status":"pong"}
```

**Step 7: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/bin/mac-fan-ctrl-helper.rs src-tauri/src/main.rs src-tauri/Cargo.toml
git commit -m "feat: integrate real SmcWriter into privileged helper binary"
```

---

### Task 5: Update AppState to Use Socket Client as Fallback

**Files:**
- Modify: `src-tauri/src/commands.rs:34-52` (AppState::new)

**Step 1: Update AppState::new() initialization order**

Replace the writer initialization in `AppState::new()`:

```rust
use crate::smc_client::SmcSocketClient;

// In AppState::new():
let writer: Option<Box<dyn SmcWriteApi>> = SmcWriter::new()
    .map(|w| Box::new(w) as Box<dyn SmcWriteApi>)
    .or_else(|direct_err| {
        warn_log!(
            "[mac-fan-ctrl] Direct SMC writer failed: {direct_err} — trying socket client"
        );
        SmcSocketClient::new()
            .map(|c| Box::new(c) as Box<dyn SmcWriteApi>)
    })
    .map_err(|e| {
        warn_log!("[mac-fan-ctrl] Socket client also failed (fan control disabled): {e}");
        e
    })
    .ok();
```

**Step 2: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: Compiles.

**Step 3: Test the fallback flow manually**

1. Start helper: `sudo cargo run --bin mac-fan-ctrl-helper`
2. Run app without sudo: `pnpm tauri dev`
3. Verify fan control works (Custom... button, set RPM)
4. Verify "Hide to menu bar" works
5. Verify "Pin on Top" works

**Step 4: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat: fall back to socket client when direct SMC writer unavailable"
```

---

### Task 6: Update Privilege Escalation for Helper Installation

**Files:**
- Modify: `src-tauri/src/commands.rs` (request_privilege_restart and new command)

**Step 1: Add install_helper command**

Add a new Tauri command that installs and starts the helper daemon:

```rust
const HELPER_SOCKET: &str = "/var/run/mac-fan-ctrl.sock";
const HELPER_INSTALL_DIR: &str = "/Library/PrivilegedHelperTools";
const LAUNCHDAEMON_DIR: &str = "/Library/LaunchDaemons";
const DAEMON_LABEL: &str = "io.github.naufaldi.mac-fan-ctrl.helper";

#[tauri::command]
pub fn install_helper(app_handle: tauri::AppHandle) -> Result<String, String> {
    // Find the helper binary — either bundled in .app or in target/debug
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Failed to get exe path: {e}"))?;

    let helper_binary = find_helper_binary(&exe_path)?;

    let install_path = format!("{HELPER_INSTALL_DIR}/{DAEMON_LABEL}");
    let plist_path = format!("{LAUNCHDAEMON_DIR}/{DAEMON_LABEL}.plist");

    let plist_content = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{DAEMON_LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{install_path}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardErrorPath</key>
    <string>/tmp/{DAEMON_LABEL}.log</string>
</dict>
</plist>"#
    );

    // Use osascript to install with admin privileges
    let shell_commands = format!(
        "mkdir -p '{HELPER_INSTALL_DIR}' && \
         cp '{}' '{install_path}' && \
         chmod 755 '{install_path}' && \
         chown root:wheel '{install_path}' && \
         echo '{}' > '{plist_path}' && \
         chown root:wheel '{plist_path}' && \
         chmod 644 '{plist_path}' && \
         launchctl bootout system/{DAEMON_LABEL} 2>/dev/null; \
         launchctl bootstrap system '{plist_path}'",
        helper_binary.to_string_lossy(),
        plist_content.replace('\'', "'\\''"),
    );

    let script = format!(
        "do shell script \"{}\" with administrator privileges",
        shell_commands.replace('\\', "\\\\").replace('"', "\\\"")
    );

    let result = std::process::Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|e| format!("Failed to launch osascript: {e}"))?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        if stderr.contains("User canceled") || stderr.contains("-128") {
            return Err("User cancelled the authorization request".to_string());
        }
        return Err(format!("Installation failed: {stderr}"));
    }

    // Wait for socket to appear
    for _ in 0..20 {
        if std::path::Path::new(HELPER_SOCKET).exists() {
            // Try connecting the SmcSocketClient and update app state
            return Ok("Helper installed and running".to_string());
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }

    Err("Helper installed but socket not found after 5 seconds".to_string())
}

fn find_helper_binary(exe_path: &std::path::Path) -> Result<std::path::PathBuf, String> {
    // Production: look in the .app bundle
    let app_bundle_helper = exe_path
        .parent() // MacOS/
        .and_then(|p| p.parent()) // Contents/
        .map(|p| p.join("MacOS/mac-fan-ctrl-helper"));

    if let Some(ref path) = app_bundle_helper {
        if path.exists() {
            return Ok(path.clone());
        }
    }

    // Dev mode: look in target/debug or target/release
    let target_dir = exe_path.parent().unwrap_or(exe_path);
    let debug_helper = target_dir.join("mac-fan-ctrl-helper");
    if debug_helper.exists() {
        return Ok(debug_helper);
    }

    Err("Helper binary not found. Build it with: cargo build --bin mac-fan-ctrl-helper".to_string())
}
```

**Step 2: Add reconnect_writer command**

After the helper is installed, the app needs to reconnect:

```rust
#[tauri::command]
pub fn reconnect_writer(state: State<'_, AppState>) -> Result<bool, String> {
    let client = SmcSocketClient::new().map_err(|e| e.to_string())?;
    let mut writer_guard = state.smc_writer.lock().map_err(|e| e.to_string())?;
    *writer_guard = Some(Box::new(client));
    Ok(true)
}
```

**Step 3: Register new commands in main.rs**

Add `commands::install_helper` and `commands::reconnect_writer` to the `invoke_handler` list in `main.rs`.

**Step 4: Add frontend wrappers**

In `src/lib/tauriCommands.ts`, add:

```typescript
export async function installHelper(): Promise<string> {
    return invoke<string>("install_helper");
}

export async function reconnectWriter(): Promise<boolean> {
    return invoke<boolean>("reconnect_writer");
}
```

**Step 5: Update the privilege banner in DesktopDashboard.svelte**

Replace `handleGrantAccess` to install the helper instead of restarting the app:

```typescript
async function handleGrantAccess(): Promise<void> {
    try {
        await installHelper();
        await reconnectWriter();
        hasWriteAccess = true;
    } catch (error) {
        const msg = error instanceof Error ? error.message : String(error);
        if (!msg.includes('cancelled') && !msg.includes('canceled')) {
            bannerMessage = msg;
        }
    }
}
```

**Step 6: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: Compiles.

**Step 7: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/main.rs src/lib/tauriCommands.ts src/components/DesktopDashboard.svelte
git commit -m "feat: add helper installation flow with admin prompt (no app restart)"
```

---

### Task 7: End-to-End Verification

**Step 1: Build the helper**

```bash
cd src-tauri && cargo build --bin mac-fan-ctrl-helper
```

**Step 2: Test the full flow without sudo**

1. Run `pnpm tauri dev` (NO sudo)
2. App shows "Fan control requires elevated privileges" banner
3. Click "Grant Access"
4. macOS admin password dialog appears
5. Enter password
6. Banner disappears, fan control works
7. Verify: "Hide to menu bar" works
8. Verify: "Pin on Top" works
9. Verify: App appears in Cmd+Tab

**Step 3: Test helper persistence**

1. Quit the app
2. Run `pnpm tauri dev` again (NO sudo)
3. Fan control should work immediately (helper already running from previous install)
4. No admin prompt needed

**Step 4: Test cleanup**

```bash
# Verify helper is running
sudo launchctl list | grep mac-fan-ctrl

# Verify socket exists
ls -la /var/run/mac-fan-ctrl.sock

# Uninstall helper
sudo launchctl bootout system/io.github.naufaldi.mac-fan-ctrl.helper
sudo rm /Library/PrivilegedHelperTools/io.github.naufaldi.mac-fan-ctrl.helper
sudo rm /Library/LaunchDaemons/io.github.naufaldi.mac-fan-ctrl.helper.plist
```

**Step 5: Final commit**

```bash
git add -A
git commit -m "feat: privileged helper daemon — eliminates sudo requirement (closes #45)"
```

---

### Summary of All Files

| File | Action | Task |
|------|--------|------|
| `src-tauri/src/smc_protocol.rs` | Create | 1 |
| `src-tauri/src/smc_client.rs` | Create | 2 |
| `src-tauri/src/smc_writer.rs` | Modify (add 2 error variants) | 2 |
| `src-tauri/src/bin/mac-fan-ctrl-helper.rs` | Create | 3, 4 |
| `src-tauri/src/lib.rs` | Create | 4 |
| `src-tauri/Cargo.toml` | Modify (add `[lib]` + `[[bin]]`) | 3, 4 |
| `src-tauri/src/main.rs` | Modify (module decls, command registration) | 1, 2, 4, 6 |
| `src-tauri/src/commands.rs` | Modify (AppState::new, install_helper, reconnect_writer) | 5, 6 |
| `src/lib/tauriCommands.ts` | Modify (add installHelper, reconnectWriter) | 6 |
| `src/components/DesktopDashboard.svelte` | Modify (update handleGrantAccess) | 6 |
