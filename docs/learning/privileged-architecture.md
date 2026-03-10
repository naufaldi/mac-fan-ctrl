# Privileged Architecture for macOS Apps

How macOS apps access hardware (like SMC) without running the entire app as root.

## The Problem

macOS protects hardware access behind privilege boundaries:

```
SMC reads  (temperature, fan speed)  → Any user can do this
SMC writes (set fan RPM, fan mode)   → Only root (EUID 0) can do this
```

Running the whole app as root (`sudo`) breaks macOS window management:
- `getCurrentWindow().hide()` fails silently
- `setAlwaysOnTop()` fails silently
- App disappears from Cmd+Tab (window server mismatch)
- Tray icon may not render correctly

## The Solution: Privilege Separation

Split the app into two processes — a **UI process** (user) and a **helper** (root):

```
┌─────────────────────────────────┐     ┌──────────────────────────┐
│  Main App (runs as USER)        │     │  Helper (runs as ROOT)   │
│                                 │     │                          │
│  ┌───────────┐  ┌────────────┐  │     │  ┌────────────────────┐  │
│  │ Svelte UI │  │ Rust       │  │ XPC │  │ SMC Write Service  │  │
│  │ (WebView) │──│ Backend    │──│────►│  │ (fan RPM, mode)    │  │
│  └───────────┘  │            │  │     │  └────────────────────┘  │
│                 │ SMC reads ✅│  │     │                          │
│                 │ (no root)   │  │     │  Auto-starts on boot     │
│                 └────────────┘  │     │  Lives in LaunchDaemons   │
└─────────────────────────────────┘     └──────────────────────────┘
          Window mgmt works ✅                Only handles writes
```

## macOS Privilege Mechanisms (Ranked)

### 1. SMJobBless + XPC (Recommended)

The Apple-sanctioned way for apps to install a privileged helper.

**How it works:**
1. App calls `SMJobBless()` — macOS shows the standard admin password dialog
2. macOS installs a small helper binary into `/Library/PrivilegedHelperTools/`
3. macOS creates a LaunchDaemon plist in `/Library/LaunchDaemons/`
4. Helper runs as root, communicates with app via XPC (Apple's IPC framework)
5. Helper persists across reboots — user only authenticates once

```
First launch:
  App ──SMJobBless()──► macOS ──"Enter admin password"──► User
                              ──installs helper──► /Library/PrivilegedHelperTools/
                              ──creates plist──► /Library/LaunchDaemons/

Every subsequent launch:
  App ──XPC connection──► Helper (already running as root)
      ──"set fan 0 to 3000 RPM"──►
      ◄──"ok"──
```

**Files involved:**
```
MyApp.app/
├── Contents/
│   ├── MacOS/MyApp              # Main app binary (user)
│   ├── Library/
│   │   └── LaunchServices/
│   │       └── io.github.myapp.helper   # Helper binary (bundled)
│   └── Info.plist               # References helper's signing identity
│
/Library/PrivilegedHelperTools/
│   └── io.github.myapp.helper   # Installed by SMJobBless (root)
/Library/LaunchDaemons/
    └── io.github.myapp.helper.plist  # Created by SMJobBless
```

**Pros:** Apple-supported, survives reboots, one-time auth, code-signed verification
**Cons:** Complex setup, requires proper code signing, XPC boilerplate

### 2. Authorization Services + authopen

Use macOS Authorization Services to get a one-time privilege token.

```rust
// Conceptual flow (C API, wrapped in Rust via FFI)
AuthorizationCreate()           // Create an auth reference
AuthorizationCopyRights(        // Request the "system.privilege.admin" right
    kAuthorizationFlagInteractionAllowed
)                               // macOS shows password dialog
// Now execute privileged operations via the auth ref
```

**Pros:** Simpler than SMJobBless, no persistent helper needed
**Cons:** Authorization expires, user re-prompted each session, not persistent

### 3. LaunchDaemon (Manual Install)

Install a helper daemon manually (no SMJobBless).

```xml
<!-- /Library/LaunchDaemons/io.github.mac-fan-ctrl.helper.plist -->
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>io.github.mac-fan-ctrl.helper</string>
    <key>ProgramArguments</key>
    <array>
        <string>/Library/PrivilegedHelperTools/mac-fan-ctrl-helper</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>
```

Install with:
```bash
sudo cp mac-fan-ctrl-helper /Library/PrivilegedHelperTools/
sudo cp io.github.mac-fan-ctrl.helper.plist /Library/LaunchDaemons/
sudo launchctl load /Library/LaunchDaemons/io.github.mac-fan-ctrl.helper.plist
```

**Pros:** Simple, persistent, no XPC required (can use Unix socket)
**Cons:** Requires manual `sudo` install step, no automatic code-sign verification

### 4. osascript (Current Approach)

What mac-fan-ctrl does today — restart the entire app as root.

```rust
let script = format!(
    "do shell script \"'{binary_path}' &>/dev/null &\" with administrator privileges"
);
std::process::Command::new("osascript").arg("-e").arg(&script).output();
```

**Pros:** Zero infrastructure, works immediately
**Cons:** Entire app runs as root, breaks window management, re-prompts every launch

## Comparison Table

| Approach | Persistent | User Prompts | Window Mgmt | Complexity | App Store |
|----------|-----------|-------------|-------------|------------|-----------|
| SMJobBless + XPC | Yes | Once | Works | High | No* |
| Authorization Services | No | Per session | Works | Medium | No |
| LaunchDaemon (manual) | Yes | Once (install) | Works | Medium | No |
| osascript (current) | No | Every launch | Broken | Low | No |

*None of these work in the Mac App Store because SMC access requires disabling App Sandbox.

## XPC Communication Deep Dive

XPC (Cross-Process Communication) is Apple's IPC framework:

```
┌──────────────┐         XPC Connection         ┌──────────────┐
│  App (user)  │ ◄──────────────────────────►  │ Helper (root) │
│              │                                │               │
│  NSXPCConnection                              │  NSXPCListener│
│  .remoteObjectProxy                           │  .delegate    │
│  .exportedInterface                           │               │
└──────────────┘                                └──────────────┘
```

**XPC Protocol (Objective-C / Swift):**
```swift
@objc protocol HelperProtocol {
    func setFanRPM(fanIndex: UInt8, rpm: Float,
                   withReply reply: @escaping (Bool, String?) -> Void)
    func setFanAuto(fanIndex: UInt8,
                    withReply reply: @escaping (Bool, String?) -> Void)
    func getFanStatus(withReply reply: @escaping ([String: Any]) -> Void)
}
```

**From Rust**, XPC can be accessed via:
- `objc2` crate (Objective-C FFI) — call NSXPCConnection directly
- Unix domain socket as a simpler alternative (no XPC dependency)

## Unix Domain Socket Alternative

Simpler than XPC, works well for Rust-to-Rust communication:

```
App (user)                              Helper (root)
    │                                       │
    │  connect("/var/run/mac-fan-ctrl.sock")│
    │──────────────────────────────────────►│
    │                                       │
    │  {"cmd":"set_rpm","fan":0,"rpm":3000} │
    │──────────────────────────────────────►│
    │                                       │ IOServiceOpen → SMC write
    │  {"ok":true}                          │
    │◄──────────────────────────────────────│
```

**Security:** Validate the connecting process via `getpeereid()` or `SO_PEERCRED` to ensure only the legitimate app connects.

```rust
// Helper side (simplified)
use std::os::unix::net::UnixListener;

let listener = UnixListener::bind("/var/run/mac-fan-ctrl.sock")?;
for stream in listener.incoming() {
    let stream = stream?;
    // Verify peer credentials
    let (uid, _gid) = get_peer_credentials(&stream)?;
    // Process commands from the app
    handle_client(stream)?;
}
```

```rust
// App side (simplified)
use std::os::unix::net::UnixStream;

let stream = UnixStream::connect("/var/run/mac-fan-ctrl.sock")?;
// Send command as JSON
serde_json::to_writer(&stream, &SmcCommand::SetRpm { fan: 0, rpm: 3000 })?;
// Read response
let response: SmcResponse = serde_json::from_reader(&stream)?;
```

## How Other Apps Do It

### Macs Fan Control (crystalidea)
- Uses a **LaunchDaemon** helper (`com.crystalidea.macsfancontrol.helper`)
- Helper installed to `/Library/PrivilegedHelperTools/`
- Communicates via XPC
- One-time admin prompt during installation

### iStat Menus (bjango)
- Uses **SMJobBless** to install privileged helper
- Helper: `com.bjango.istatmenus.helper`
- XPC for IPC
- Helper auto-updates when main app updates

### TG Pro (tunabelly)
- Similar SMJobBless + XPC pattern
- Helper handles all SMC writes
- Main app is a standard menu bar app (user context)

## Recommended Approach for mac-fan-ctrl

**Unix Domain Socket + LaunchDaemon** — simpler than XPC, pure Rust, no Objective-C:

```
Phase 1: Create the helper binary
  └── New Rust binary: mac-fan-ctrl-helper
  └── Moves smc_writer.rs logic into the helper
  └── Listens on /var/run/mac-fan-ctrl.sock
  └── Accepts JSON commands, returns JSON responses

Phase 2: Create the socket client in the main app
  └── New module: src-tauri/src/smc_client.rs
  └── Implements SmcWriteApi trait over Unix socket
  └── Replaces direct SmcWriter when helper is available

Phase 3: Helper installation flow
  └── On first launch: detect if helper is installed
  └── If not: prompt user with admin dialog (osascript)
  └── Install helper binary + LaunchDaemon plist
  └── Verify helper is running

Phase 4: Remove sudo requirement
  └── Main app always runs as user
  └── Fan reads: direct (no change)
  └── Fan writes: routed through helper socket
  └── Dev mode: helper serves both dev and production
```

## Security Considerations

1. **Socket permissions**: Set socket file to `0600` owned by root — only root helper can create it, but any local user can connect (validate via peer credentials)
2. **Command validation**: Helper must validate all inputs (fan index bounds, RPM limits, temperature safety checks) — never trust the client
3. **Code signing**: In production, helper should verify the connecting app's code signature
4. **Fail-safe**: Helper must restore fans to auto on crash/exit (same Drop impl as current SmcWriter)
5. **Rate limiting**: Prevent rapid-fire SMC writes that could confuse the hardware

## Key Concepts Glossary

| Term | Meaning |
|------|---------|
| **EUID** | Effective User ID — determines what the process can access (0 = root) |
| **IOKit** | Apple's framework for communicating with hardware drivers |
| **SMC** | System Management Controller — the chip that controls fans, LEDs, power |
| **XPC** | Apple's inter-process communication framework |
| **LaunchDaemon** | A system-level background service managed by `launchd` (runs as root) |
| **LaunchAgent** | A per-user background service (runs as logged-in user) |
| **SMJobBless** | API to install a privileged helper with admin authorization |
| **Unix Domain Socket** | Local-only socket for IPC — like a network socket but filesystem-based |
| **Privilege Separation** | Security pattern: split app into privileged and unprivileged parts |

## Further Reading

- [Apple: Authorization Services Programming Guide](https://developer.apple.com/library/archive/documentation/Security/Conceptual/authorization_concepts/)
- [Apple: Creating a Launch Daemon](https://developer.apple.com/library/archive/documentation/MacOSX/Conceptual/BPSystemStartup/Chapters/CreatingLaunchdJobs.html)
- [Apple: XPC Services](https://developer.apple.com/library/archive/documentation/MacOSX/Conceptual/BPSystemStartup/Chapters/DesigningDaemons.html)
- [SMJobBless sample code](https://developer.apple.com/documentation/servicemanagement/updating-your-app-package-installer-to-use-the-new-service-management-api)
- [vladkens/macmon](https://github.com/vladkens/macmon) — Rust macOS monitoring (uses IOReport)
- [crystalidea/macs-fan-control](https://github.com/nickswalker/macos-fan-control) — Reference implementation
