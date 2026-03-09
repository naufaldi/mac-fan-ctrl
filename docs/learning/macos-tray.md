# macOS Menu Bar Tray Icons in Tauri v2

This document explains how the macOS menu bar (system tray) works, how Tauri v2 wraps the native APIs, and how mac-fan-ctrl uses all of this to show live CPU temperatures and fan controls directly in the menu bar. If you're a frontend developer, every concept is mapped to a web equivalent you already know.

## 1) What is the macOS Menu Bar?

The menu bar is the strip at the top of your screen. The right side holds **status items** — small icons and text that apps place there (Wi-Fi, battery, clock). Our app puts a fan icon + CPU temperature there.

### Web-to-Native Concept Map

| Web Concept | macOS Equivalent | What It Does |
|---|---|---|
| Fixed navbar / header bar | `NSStatusBar` | The menu bar itself — always visible at screen top |
| Navbar icon/button | `NSStatusItem` | One slot in the right side of the menu bar |
| Dropdown menu | `NSMenu` | The menu that appears when you click a status item |
| Menu item / `<li>` | `NSMenuItem` | One row in the dropdown |
| `display: none` on app | `ActivationPolicy::Accessory` | App runs with no Dock icon — menu bar only |
| `window.close()` override | `prevent_close()` + `hide()` | Close button hides the window instead of quitting |
| Badge text on favicon | `setTitle("42°C")` | Text shown next to the icon in the menu bar |

### Where the Tray Icon Lives

```
┌──────────────────────────────────────────────────────────────────────┐
│ ◉ Finder  File  Edit  View │          Wi-Fi  🔋  🔊  🌡42°C  10:30 │
│            App Menu Bar     │          ← Status Items (right side) → │
└──────────────────────────────────────────────────────────────────────┘
                                                      ▲
                                                      │
                                               Our tray icon
                                             (icon + "42°C" title)
                                                      │
                                                      ▼
                                         ┌─────────────────────────┐
                                         │ Show Mac Fan Control    │
                                         │─────────────────────────│
                                         │ Available fans:         │
                                         │  ▸ Left Fan – Auto      │
                                         │  ▸ Right Fan – 3200 RPM │
                                         │─────────────────────────│
                                         │ Fan presets:            │
                                         │  ☑ Silent               │
                                         │  ☐ Performance          │
                                         │─────────────────────────│
                                         │ Quit Mac Fan Control    │
                                         └─────────────────────────┘
```

### Key macOS Concepts

- **NSStatusBar**: The system-wide menu bar. There's only one. Apps request a slot in it.
- **NSStatusItem**: Your app's slot. It holds an icon, optional text (title), and a menu.
- **Template images**: Icons provided as black-on-transparent PNGs. macOS automatically inverts them for dark mode — you don't handle light/dark yourself.
- **Menu bar vs Dock**: Most apps show in the Dock. Menu bar apps (`Accessory` policy) hide from the Dock entirely and live only in the status area.

## 2) How Tauri Wraps NSStatusItem

You never write Objective-C. Tauri provides a Rust API that maps to native Cocoa calls through FFI (Foreign Function Interface).

### Three-Layer Abstraction

```
┌─────────────────────────────────────────────┐
│              Your Rust Code                 │
│   TrayIconBuilder::new()                    │
│     .icon(image)                            │
│     .title("42°C")                          │
│     .menu(&menu)                            │
│     .build(app)                             │
├─────────────────────────────────────────────┤
│          Tauri Internals (tao/wry)          │
│   Creates NSStatusItem, sets NSImage,       │
│   attaches NSMenu, installs event delegate  │
├─────────────────────────────────────────────┤
│            macOS Cocoa (AppKit)             │
│   NSStatusBar.systemStatusBar()             │
│     .statusItem(withLength: .variable)      │
│   NSStatusItem.button.image = nsImage       │
│   NSStatusItem.menu = nsMenu                │
└─────────────────────────────────────────────┘
```

### API Quick Reference

| Tauri Method | macOS Equivalent | Purpose |
|---|---|---|
| `TrayIconBuilder::new()` | `NSStatusBar.statusItem(withLength:)` | Create a status item slot |
| `.icon(image)` | `NSStatusItem.button.image` | Set the menu bar icon |
| `.icon_as_template(true)` | `NSImage.isTemplate = true` | Enable auto light/dark inversion |
| `.title("42°C")` | `NSStatusItem.button.title` | Text shown next to icon |
| `.tooltip("Mac Fan Control")` | `NSStatusItem.button.toolTip` | Hover tooltip |
| `.menu(&menu)` | `NSStatusItem.menu` | Attach dropdown menu |
| `.show_menu_on_left_click(true)` | Delegate behavior | Left-click opens menu (vs right-click only) |
| `.on_menu_event(handler)` | `NSMenuDelegate` | Callback when menu item is selected |
| `.on_tray_icon_event(handler)` | `NSStatusBarButton` target/action | Callback for icon clicks |
| `tray.set_title(Some("42°C"))` | `button.title = "42°C"` | Update title at runtime |
| `tray.set_menu(Some(menu))` | `statusItem.menu = menu` | Replace entire menu at runtime |

## 3) Key Files

| File | Role |
|---|---|
| `src-tauri/src/tray.rs` | All tray logic: setup, menu building, guard, events, updates |
| `src-tauri/src/main.rs` | Calls `setup_tray()`, runs sensor stream that feeds tray updates |
| `src-tauri/src/commands.rs` | `TrayHandle` newtype wrapper (line 17), `AppState` struct (lines 21-25) |
| `src-tauri/icons/menu-icon-template@2x.png` | The fan icon asset (32x32 template image) |

## 4) Tray Setup: How It All Starts

The tray is created during Tauri's `setup` phase — think of it like React's `useEffect(() => { ... }, [])` that runs once on mount.

### `setup_tray()` — Line by Line

From `src-tauri/src/tray.rs:62-78`:

```rust
pub fn setup_tray(app: &mut tauri::App) -> Result<TrayIcon, tauri::Error> {
    // 1. Load the icon at compile time (web analogy: webpack asset import,
    //    but the bytes are baked into the binary — no runtime file loading)
    let icon_bytes = include_bytes!("../icons/menu-icon-template@2x.png");
    let icon = Image::from_bytes(icon_bytes)?;

    // 2. Build a simple initial menu (just "Show" + "Quit")
    //    The full fan menu comes later once sensor data arrives
    let initial_menu = build_initial_menu(app.handle())?;

    // 3. Assemble the tray icon
    TrayIconBuilder::new()
        .icon(icon)                           // Fan icon
        .icon_as_template(true)               // macOS auto-inverts for dark mode
        .title("--°C")                        // Placeholder until first sensor read
        .tooltip("Mac Fan Control")           // Hover text
        .menu(&initial_menu)                  // Attach dropdown
        .show_menu_on_left_click(true)        // Left-click opens menu
        .on_menu_event(handle_menu_event)     // Menu item selected callback
        .on_tray_icon_event(handle_tray_icon_event) // Icon click callback
        .build(app)                           // Register with macOS
}
```

### Registration in `main.rs`

From `src-tauri/src/main.rs:271-283`:

```rust
// Initialize menu bar tray icon
match tray::setup_tray(app) {
    Ok(tray_icon) => {
        // Store the TrayIcon handle in Tauri's managed state
        // so other code can call set_title() / set_menu() later
        app.manage(commands::TrayHandle(tray_icon));
    }
    Err(e) => {
        warn_log!("[mac-fan-ctrl] Tray setup failed: {e}");
    }
}

// Hide Dock icon — app lives in the menu bar
#[cfg(target_os = "macos")]
app.set_activation_policy(tauri::ActivationPolicy::Accessory);
```

**`ActivationPolicy::Accessory`** is like running a service worker with no tab. The app has no Dock icon, no app-switcher entry — it exists only in the menu bar. After setting this policy, macOS deactivates the app, so we immediately re-show and re-focus the main window (lines 287-290).

## 5) Menu Architecture

### Two-Phase Approach

The tray menu is built in two phases:

1. **Initial static menu** (`build_initial_menu`): Just "Show Mac Fan Control" + separator + "Quit". Created at startup before any sensor data exists.
2. **Dynamic rebuilt menu** (`build_tray_menu`): Full menu with fans, presets, and controls. Rebuilt every 3 seconds when new sensor data arrives.

### Full Dropdown Structure

```
┌───────────────────────────────────────────┐
│ Show Mac Fan Control          [MenuItem]  │
│───────────────────────────── [Separator]  │
│ Available fans:       [MenuItem/disabled] │
│  ▸ Left Fan – Auto            [Submenu]  │──┐
│  ▸ Right Fan – 3200 RPM       [Submenu]  │  │
│───────────────────────────── [Separator]  │  │
│ Fan presets:          [MenuItem/disabled] │  │
│  ☑ Silent              [CheckMenuItem]   │  │
│  ☐ Performance         [CheckMenuItem]   │  │
│  ☐ Full Blast          [CheckMenuItem]   │  │
│───────────────────────────── [Separator]  │  │
│ Quit Mac Fan Control          [MenuItem]  │  │
└───────────────────────────────────────────┘  │
                                               │
  Fan submenu expands to: ◀────────────────────┘
  ┌──────────────────────────┐
  │ ☑ Auto     [CheckMenuItem] │
  │ ☐ 1500 RPM [CheckMenuItem] │
  │ ☐ 3000 RPM [CheckMenuItem] │
  │ ☐ 4500 RPM [CheckMenuItem] │
  │ ☐ 6000 RPM [CheckMenuItem] │
  └──────────────────────────┘
```

### Menu Item ID Encoding

Every menu item gets a string ID that encodes both the action type and the target. Think of it like encoding data in HTML `data-*` attributes, but for native menus.

| Constant | Prefix / Value | Example ID | Decoded Meaning |
|---|---|---|---|
| `SHOW_WINDOW` | `"show_window"` | `show_window` | Open main window |
| `QUIT` | `"quit"` | `quit` | Quit the app |
| `PRESET_PREFIX` | `"preset::"` | `preset::Silent` | Apply the "Silent" preset |
| `FAN_AUTO_PREFIX` | `"fan_auto::"` | `fan_auto::0` | Set fan 0 to auto mode |
| `FAN_RPM_PREFIX` | `"fan_rpm::"` | `fan_rpm::1::3000` | Set fan 1 to 3000 RPM |

The `fan_rpm::` prefix uses a double-encoded format: `fan_rpm::{fan_index}::{rpm}`. This is parsed with `split_once("::")` in the event handler.

### Building a Fan Submenu

From `src-tauri/src/tray.rs:148-183`:

```rust
fn build_fan_submenu(
    app: &AppHandle,
    fan: &FanData,
) -> Result<tauri::menu::Submenu<tauri::Wry>, tauri::Error> {
    // Title shows current mode: "Left Fan – Auto" or "Left Fan – 3200 RPM"
    let mode_label = match fan.mode {
        FanMode::Auto => "Auto".to_string(),
        FanMode::Forced => format!("{} RPM", fan.target as u32),
    };
    let title = format!("{} – {mode_label}", fan.label);

    // "Auto" checkbox — checked when fan is in auto mode
    let auto_id = format!("{FAN_AUTO_PREFIX}{}", fan.index);
    let is_auto = matches!(fan.mode, FanMode::Auto);
    let auto_item = CheckMenuItem::with_id(app, &auto_id, "Auto", true, is_auto, None::<&str>)?;

    // RPM steps: 25%, 50%, 75%, 100% of the fan's max RPM
    let rpm_ratios = [0.25_f32, 0.50, 0.75, 1.00];
    let rpm_items: Vec<CheckMenuItem<tauri::Wry>> = rpm_ratios
        .iter()
        .filter_map(|ratio| {
            let rpm = (fan.max * ratio) as u32;
            let id = format!("{FAN_RPM_PREFIX}{}::{rpm}", fan.index);
            let label = format!("{rpm} RPM");
            // Check if this RPM is the currently forced target (within 50 RPM tolerance)
            let checked =
                matches!(fan.mode, FanMode::Forced) && (fan.target as u32).abs_diff(rpm) < 50;
            CheckMenuItem::with_id(app, &id, &label, true, checked, None::<&str>).ok()
        })
        .collect();

    // Assemble: Auto + RPM options
    let mut sub = SubmenuBuilder::new(app, &title).item(&auto_item);
    for rpm_item in &rpm_items {
        sub = sub.item(rpm_item);
    }
    sub.build()
}
```

## 6) Live Updates: Keeping the Tray Current

The sensor stream in `main.rs` drives tray updates on a background thread, like a `setInterval` that ticks every second.

### Data Flow

```
┌──────────────┐     1s interval     ┌──────────────────┐
│  SMC Hardware │ ──────────────────▸ │  SensorService   │
│  (temp/fans)  │                     │  read_fans_only  │
└──────────────┘                     │  read_all_sensors│
                                      └────────┬─────────┘
                                               │
                              ┌────────────────┼────────────────┐
                              ▼                                 ▼
                    Every cycle (1s)                   Every 3rd cycle (3s)
                 ┌───────────────────┐           ┌────────────────────────┐
                 │ update_tray_title │           │   update_tray_menu     │
                 │ set_title("42°C") │           │ rebuild entire NSMenu  │
                 │    (fast, ~0ms)   │           │  with fan/preset data  │
                 └───────────────────┘           │   (slower, ~5-10ms)   │
                                                 └────────────────────────┘
```

### Fast Path: Title Only (Every 1 Second)

From `src-tauri/src/tray.rs:187-199`:

```rust
pub fn update_tray_title(app_handle: &AppHandle, sensor_data: &SensorData) {
    // Extract CPU package temp, format as "42°C", fallback to "--°C"
    let cpu_temp_str = sensor_data
        .summary
        .cpu_package
        .as_ref()
        .and_then(|s| s.value)
        .map(|v| format!("{:.0}°C", v))
        .unwrap_or_else(|| "--°C".to_string());

    // try_state() — returns None if TrayHandle hasn't been registered yet
    if let Some(tray_state) = app_handle.try_state::<TrayHandle>() {
        let _ = tray_state.0.set_title(Some(&cpu_temp_str));
    }
}
```

This is cheap — it only updates the text next to the icon. No menu rebuild.

### Full Path: Title + Menu Rebuild (Every 3 Seconds)

From `src-tauri/src/tray.rs:202-248`:

```rust
pub fn update_tray_menu(app_handle: &AppHandle, sensor_data: &SensorData) {
    // Skip if menu guard is active (user has the dropdown open)
    if is_menu_guarded() {
        return;
    }

    let Some(tray_state) = app_handle.try_state::<TrayHandle>() else {
        return;
    };

    // Read current preset state
    let app_state = app_handle.state::<AppState>();
    let active_preset = app_state
        .preset_store
        .lock()
        .ok()
        .and_then(|s| s.active_preset.clone());

    // ... gather fan data and presets ...

    // Build entirely new menu and swap it in
    match build_tray_menu(app_handle, &sensor_data.fans, active_preset.as_deref(), &all_presets) {
        Ok(menu) => {
            let _ = tray_state.0.set_menu(Some(menu));
        }
        Err(e) => {
            warn_log!("[tray] build_tray_menu FAILED: {e}");
        }
    }
}
```

**Why rebuild the entire menu?** macOS's `NSMenu` has no item-level patching API. You can't do `menu.items[2].title = "new text"` efficiently. The web analogy: imagine if the DOM had no virtual DOM, no `innerHTML` — your only option is to throw away the entire `<ul>` and create a new one. That's what `set_menu()` does.

## 7) Menu Guard: Preventing Dropdown Dismissal

### The Problem

Every time you call `set_menu()`, macOS dismisses the open dropdown. Since we rebuild the menu every 3 seconds, the user would see their dropdown close and reopen constantly — unusable.

### The Solution: AtomicU64 Timestamp Guard

From `src-tauri/src/tray.rs:13-44`:

```rust
/// Timestamp (millis since epoch) of last tray icon click.
static LAST_TRAY_CLICK_MS: AtomicU64 = AtomicU64::new(0);

/// How long (ms) to guard the menu from rebuilds after a click.
const MENU_GUARD_MS: u64 = 15_000;

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn is_menu_guarded() -> bool {
    let last_click = LAST_TRAY_CLICK_MS.load(Ordering::Relaxed);
    if last_click == 0 {
        return false;
    }
    let elapsed = now_millis().saturating_sub(last_click);
    elapsed < MENU_GUARD_MS
}

fn mark_menu_opened() {
    LAST_TRAY_CLICK_MS.store(now_millis(), Ordering::Relaxed);
}

fn mark_menu_closed() {
    LAST_TRAY_CLICK_MS.store(0, Ordering::Relaxed);
}
```

### State Transitions

| Event | Action | Guard State |
|---|---|---|
| User clicks tray icon | `mark_menu_opened()` → stores current timestamp | **Guarded** — menu rebuilds skip |
| User selects a menu item | `mark_menu_closed()` → stores 0 | **Unguarded** — rebuilds resume |
| 15 seconds pass since click | `is_menu_guarded()` returns false (elapsed ≥ 15s) | **Expired** — rebuilds resume |
| No click has occurred | Timestamp is 0, `is_menu_guarded()` short-circuits | **Unguarded** |

### Why AtomicU64 Over Mutex?

`AtomicU64` is a lock-free primitive. It's accessed from two threads:
- The **sensor stream thread** checks `is_menu_guarded()` every 3 seconds
- The **main thread** calls `mark_menu_opened()` / `mark_menu_closed()` on UI events

A `Mutex` would work, but it's heavier — it can block, it can poison. `AtomicU64` is a single CPU instruction (`load` / `store`), zero overhead, no deadlock risk. For a simple timestamp flag, atomics are the right tool.

## 8) Event Handling

### Two Event Flows

```
┌─────────────────────┐         ┌──────────────────────────────┐
│   TrayIconEvent     │         │       MenuEvent              │
│   (icon clicks)     │         │   (menu item selections)     │
├─────────────────────┤         ├──────────────────────────────┤
│ Click               │         │ id = "show_window"           │
│  → mark_menu_opened │         │  → show_main_window()        │
│                     │         │                              │
│ DoubleClick         │         │ id = "quit"                  │
│  → show_main_window │         │  → quit_app()                │
│                     │         │                              │
│ Enter / Leave       │         │ id = "preset::Silent"        │
│  → debug logging    │         │  → apply_preset_from_tray()  │
│                     │         │                              │
│ Move                │         │ id = "fan_auto::0"           │
│  → (ignored)        │         │  → set_fan_auto_from_tray()  │
│                     │         │                              │
│                     │         │ id = "fan_rpm::1::3000"      │
│                     │         │  → set_fan_rpm_from_tray()   │
└─────────────────────┘         └──────────────────────────────┘
```

### Prefix-Based Dispatch

From `src-tauri/src/tray.rs:252-279`:

```rust
fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    let id = event.id().as_ref();
    mark_menu_closed();  // Menu item selected → dropdown is now closed

    match id {
        SHOW_WINDOW => show_main_window(app),
        QUIT => quit_app(app),

        // Prefix matching — like routing with path parameters
        _ if id.starts_with(PRESET_PREFIX) => {
            let preset_name = &id[PRESET_PREFIX.len()..];
            apply_preset_from_tray(app, preset_name);
        }
        _ if id.starts_with(FAN_AUTO_PREFIX) => {
            if let Ok(fan_index) = id[FAN_AUTO_PREFIX.len()..].parse::<u8>() {
                set_fan_auto_from_tray(app, fan_index);
            }
        }
        _ if id.starts_with(FAN_RPM_PREFIX) => {
            let rest = &id[FAN_RPM_PREFIX.len()..];
            if let Some((idx_str, rpm_str)) = rest.split_once("::") {
                if let (Ok(fan_index), Ok(rpm)) = (idx_str.parse::<u8>(), rpm_str.parse::<f32>()) {
                    set_fan_rpm_from_tray(app, fan_index, rpm);
                }
            }
        }
        _ => {}
    }
}
```

### Why Actions Spawn Threads

Fan control actions like `apply_preset_from_tray()` and `set_fan_rpm_from_tray()` use `std::thread::spawn`. Web analogy: this is like using a Web Worker to avoid blocking the UI thread.

```rust
fn set_fan_auto_from_tray(app: &AppHandle, fan_index: u8) {
    let app = app.clone();  // Clone the handle — it's reference-counted (like Arc)

    std::thread::spawn(move || {
        // SMC writes can take 5-50ms — too slow for the main thread
        let state = app.state::<AppState>();
        let writer_guard = match state.smc_writer.lock() { ... };
        let _ = control.set_auto(fan_index, writer);
    });
}
```

If these ran on the main thread, the menu bar would stutter every time you clicked a fan control. By spawning a thread, the UI stays responsive and the SMC write happens in the background.

### Quit: Safety First

From `src-tauri/src/tray.rs:319-329`:

```rust
fn quit_app(app: &AppHandle) {
    let state = app.state::<AppState>();
    // SAFETY: Always restore fans to auto before exit.
    // If we crash with fans forced to low RPM, the CPU could overheat.
    if let (Ok(writer_guard), Ok(mut control)) =
        (state.smc_writer.lock(), state.fan_control.lock())
    {
        if let Some(writer) = writer_guard.as_deref() {
            control.restore_all_auto(writer);
        }
    }
    app.exit(0);
}
```

## 9) State Management

### TrayHandle: A Newtype Wrapper

From `src-tauri/src/commands.rs:17`:

```rust
pub struct TrayHandle(pub tauri::tray::TrayIcon);
```

This is a **newtype** — a single-field wrapper struct. Web analogy: it's like creating a React Context provider that holds a single value. The reason for the wrapper is that Tauri's `.manage()` uses the type as the key. You can't register two `TrayIcon`s, but you can register one `TrayHandle`.

### Accessing State: `state()` vs `try_state()`

| Method | Behavior | Use When |
|---|---|---|
| `app.state::<AppState>()` | Panics if not registered | State is guaranteed to exist (AppState is always registered) |
| `app.try_state::<TrayHandle>()` | Returns `Option` | State might not exist yet (tray setup could have failed) |

Web analogy: `state()` is like `useContext()` that throws if no provider exists. `try_state()` is like a safe version that returns `null`.

### Thread Safety: Why Mutex

`AppState` wraps its fields in `Mutex`:

```rust
pub struct AppState {
    pub fan_control: Mutex<FanControlState>,
    pub smc_writer: Mutex<Option<Box<dyn SmcWriteApi>>>,
    pub preset_store: Mutex<PresetStore>,
}
```

Multiple threads access this state simultaneously:
- The **sensor stream thread** reads fan data every second
- The **main thread** handles menu events
- **Spawned threads** execute fan control actions

`Mutex` ensures only one thread accesses the data at a time. The pattern is always: `state.field.lock().ok()` → do work → lock is released when the guard goes out of scope.

## 10) App Lifecycle: Close-to-Tray and Exit Paths

### Close-to-Tray

From `src-tauri/src/main.rs:322-327`:

```rust
.on_window_event(|window, event| match event {
    tauri::WindowEvent::CloseRequested { api, .. } => {
        // Web analogy: event.preventDefault() on window.onbeforeunload
        api.prevent_close();
        let _ = window.hide();
    }
    // ...
})
```

When the user clicks the red close button, the window hides instead of closing. The app keeps running in the menu bar. This is standard macOS behavior for tray apps.

### Show from Tray

From `src-tauri/src/tray.rs:311-317`:

```rust
fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.unminimize();
    }
}
```

### Exit Paths: All Restore Fans

Every exit path restores fans to automatic mode. This is critical safety — if the app exits with fans forced to a low RPM, the CPU could overheat.

| Exit Path | Where | Restores Fans? |
|---|---|---|
| Tray → "Quit" | `tray.rs:319-329` | Yes — `control.restore_all_auto(writer)` |
| Window destroyed | `main.rs:328-330` | Yes — `restore_fans(app_handle)` |
| `ExitRequested` event | `main.rs:335-338` | Yes — `restore_fans(app_handle)` |
| SIGTERM / SIGINT | `main.rs:293-300` | Yes — signal handler calls `restore_fans()` |

## 11) Template Icons: How macOS Auto-Theming Works

### How Template Images Work

macOS template images use a simple contract:
- You provide a **black + alpha** PNG (black pixels with varying opacity)
- macOS renders it against the current menu bar appearance
- In light mode: black icon on light background
- In dark mode: macOS inverts to white icon on dark background

You do **not** provide separate light/dark variants. The system handles it.

### Icon Specification

| Property | Requirement |
|---|---|
| Format | PNG with alpha channel |
| Colors | Pure black (#000000) only — no grays, no colors |
| Size @1x | 16×16 pixels |
| Size @2x | 32×32 pixels (Retina) |
| Naming | `*-template.png` or `*-template@2x.png` |
| API flag | `.icon_as_template(true)` in `TrayIconBuilder` |

The `@2x` suffix is the Retina variant. Our codebase only includes the `@2x` version (`menu-icon-template@2x.png`) and Tauri/macOS downscales for non-Retina displays.

## 12) Common Pitfalls

| Pitfall | Symptom | Solution | Code Reference |
|---|---|---|---|
| Menu dismissed on rebuild | Dropdown closes every 3 seconds | Use timestamp guard to skip `set_menu()` while menu is open | `tray.rs:13-44` |
| Tray not ready at startup | `set_title()` silently fails | Use `try_state::<TrayHandle>()` to check if registered | `tray.rs:196` |
| Blocking main thread | Menu bar freezes on fan action | Spawn `std::thread` for SMC writes | `tray.rs:335, 378, 398` |
| Dock icon visible | App shows in Dock + menu bar | Set `ActivationPolicy::Accessory` | `main.rs:283` |
| Window gone after Accessory | Window disappears when hiding Dock icon | Re-show + re-focus window after policy change | `main.rs:287-290` |
| Icon doesn't invert in dark mode | Icon is always black (invisible in dark mode) | Use pure black PNG + `icon_as_template(true)` | `tray.rs:63-64, 70` |
| Fans stuck after crash | Fans remain at forced RPM | Orphan recovery at startup + signal handlers | `main.rs:215-260, 293-300` |

## 13) Next Steps

Now that you understand the tray system, explore these related docs:

- [Read Sensor Architecture](./read-sensor.md) — How sensor data flows from SMC to the UI (the data that feeds the tray)
- [macOS SMC](./macos-smc.md) — The hardware layer underneath: how temperature and fan readings work
- [Tauri Architecture](./tauri-architecture.md) — The full Tauri + Rust + Svelte stack overview
- [M1 Pro Fan Control](./fan-control-m1-pro-investigation.md) — Real-world investigation of Apple Silicon fan write behavior
