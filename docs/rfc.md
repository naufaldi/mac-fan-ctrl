# Request for Comments: mac-fan-ctrl Technical Design

## 1. Summary

mac-fan-ctrl is a macOS system monitoring and fan control application built with **Tauri v2** (Rust backend + Web frontend). It communicates with Apple's System Management Controller (SMC) to read temperature sensors and fan speeds, displaying real-time data through a native menu bar interface. The architecture prioritizes minimal resource usage (<0.1% CPU when idle) while providing responsive real-time monitoring. Scope is phased: **Phase A** delivers read-only monitoring and **Phase B** introduces guarded fan control modes (**Auto**, **Custom (Fixed)**, **Custom (Sensor-based)**) with thermal safety constraints.

---

## 2. Motivation

### 2.1 Why Build This?

Existing solutions like Macs Fan Control work well but are closed-source or built on older technologies. We want:
1. **Open source** alternative that the community can audit and extend
2. **Modern tech stack** that is lightweight and maintainable
3. **Learning opportunity** to explore Rust system programming on macOS

### 2.2 Why Tauri + Rust?

| Approach | Pros | Cons |
|----------|------|------|
| **Tauri + Rust** | Native performance, small bundle size, memory safety, modern web UI | Learning curve for Rust |
| **Electron + Node.js** | Familiar web tech, large ecosystem | High memory usage, large bundle, slower |
| **Native Swift** | Full macOS integration, lowest resource usage | macOS only, harder to maintain |
| **Flutter** | Cross-platform, modern UI | Heavy runtime, not truly native |

**Decision**: Tauri v2 + Rust offers the best balance of:
- Resource efficiency (Rust's zero-cost abstractions)
- UI flexibility (web technologies)
- Small bundle size (Tauri's minimal runtime)
- Safety (Rust's memory guarantees for system-level SMC access)

---

## 3. Technical Architecture

### 3.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         User Interface                           │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────────────────┐  │
│  │  Menu Bar   │  │ Main Window  │  │    Settings Panel       │  │
│  │   (Tray)    │  │  (Svelte)    │  │      (Svelte)           │  │
│  └──────┬──────┘  └──────┬───────┘  └───────────┬─────────────┘  │
│         │                │                      │                │
│         └────────────────┴──────────────────────┘                │
│                          │                                       │
│              Tauri Bridge│(Commands + Events)                   │
└──────────────────────────┼───────────────────────────────────────┘
                           │
┌──────────────────────────┼───────────────────────────────────────┐
│              Rust Backend│(src-tauri/)                           │
│                          │                                       │
│  ┌───────────────────────┴───────────────────────────────────┐  │
│  │                    Command Handler Layer                     │  │
│  │         (Exposes functions to frontend via Tauri)            │  │
│  └───────────────────────┬───────────────────────────────────┘  │
│                          │                                       │
│  ┌───────────────────────┴───────────────────────────────────┐  │
│  │                  SMC Interface Module                        │  │
│  │     ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │  │
│  │     │  Fan Reader  │  │  Temp Reader │  │  Fan Control │  │  │
│  │     │  (Read RPM)  │  │ (CPU, GPU)   │  │  (Set Speed) │  │  │
│  │     └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  │  │
│  │            │                 │                 │         │  │
│  │            └─────────────────┴─────────────────┘         │  │
│  │                          │                               │  │
│  │                    macsmc crate                         │  │
│  └──────────────────────────┼───────────────────────────────┘  │
│                             │                                   │
│  ┌──────────────────────────┴───────────────────────────────┐  │
│  │               Background Monitor Service                  │  │
│  │         (Continuous polling, data broadcasting)          │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                           │
                           ▼
              ┌────────────────────┐
              │   macOS SMC (Chip)   │
              │  (System Management   │
              │    Controller)         │
              └────────────────────┘
```

High-level behavior follows the PRD phase model:
- **Phase A (MVP)**: read-only telemetry path (SMC read -> Rust monitor service -> Tauri events -> UI).
- **Phase B (Post-MVP)**: controlled write path for fan settings (UI intent -> validated Rust command -> SMC write), with safety gates and automatic fallback to Auto mode on failure/exit.

### 3.2 Component Responsibilities

| Component | Technology | Responsibility |
|-----------|------------|----------------|
| **Frontend UI** | Svelte + TypeScript | Render temperature graphs, fan displays, user controls |
| **Tauri Bridge** | Tauri v2 API | IPC between frontend and Rust backend |
| **Command Handler** | Rust | Process frontend requests, return data |
| **SMC Interface** | Rust + macsmc crate | Low-level SMC communication |
| **Monitor Service** | Rust (async/tokio) | Background data polling and broadcasting |
| **Design System Layer** | Svelte + CSS variables | Consistent spacing, color states, typography, and dark/light parity |

---

## 4. Detailed Design

### 4.1 Rust Backend (src-tauri/)

#### 4.1.1 SMC Interface Module

```rust
// src-tauri/src/smc.rs
pub struct SmcClient {
    connection: Smc,  // from macsmc crate
}

impl SmcClient {
    /// Connect to SMC chip
    pub fn connect() -> Result<Self, SmcError>;
    
    /// Read all fan speeds (RPM)
    pub fn read_fan_speeds(&self) -> Result<Vec<FanData>, SmcError>;
    
    /// Read all temperature sensors
    pub fn read_temperatures(&self) -> Result<TemperatureData, SmcError>;
    
    /// Set fan speed (Phase B)
    pub fn set_fan_speed(&mut self, fan_id: u8, rpm: u32) -> Result<(), SmcError>;
    
    /// Return fan to automatic control (Phase B)
    pub fn set_fan_auto(&mut self, fan_id: u8) -> Result<(), SmcError>;
}

pub struct FanData {
    pub id: u8,
    pub name: String,      // "Left Fan", "Right Fan"
    pub current_rpm: u32,
    pub target_rpm: Option<u32>,  // if manually set
    pub max_rpm: u32,
}

pub struct TemperatureData {
    pub cpu: CpuTemps,
    pub gpu: Option<GpuTemps>,
    pub battery: Option<f64>,
    pub ssd: Option<f64>,
}
```

#### 4.1.2 Monitor Service

The monitor service runs continuously in a background thread, polling the SMC at regular intervals and broadcasting updates to the frontend.

```rust
// src-tauri/src/monitor.rs
pub struct MonitorService {
    smc: Arc<Mutex<SmcClient>>,
    subscribers: Vec<Callback>,
    interval: Duration,
}

impl MonitorService {
    /// Start monitoring loop
    pub async fn start(&self) {
        loop {
            let data = self.poll_sensors().await;
            self.broadcast(data);
            tokio::time::sleep(self.interval).await;
        }
    }
    
    /// Poll all sensors
    async fn poll_sensors(&self) -> SensorData {
        let smc = self.smc.lock().await;
        SensorData {
            fans: smc.read_fan_speeds(),
            temps: smc.read_temperatures(),
            timestamp: Instant::now(),
        }
    }
    
    /// Subscribe to updates (for frontend)
    pub fn subscribe(&mut self, callback: Callback);
}
```

**Polling Strategy:**
- Menu bar mode (window closed): Poll every 2-3 seconds
- Window open mode: Poll every 1 second for smoother graphs
- Adaptive: Slow down polling if no UI is consuming data

#### 4.1.3 Command Handlers (Tauri Commands)

```rust
// src-tauri/src/commands.rs

/// Get current fan speeds
#[tauri::command]
async fn get_fan_speeds(state: State<'_, AppState>) -> Result<Vec<FanData>, String>;

/// Get current temperatures
#[tauri::command]
async fn get_temperatures(state: State<'_, AppState>) -> Result<TemperatureData, String>;

/// Set manual fan speed (Phase B)
#[tauri::command]
async fn set_fan_speed(
    state: State<'_, AppState>,
    fan_id: u8,
    rpm: u32
) -> Result<(), String>;

/// Subscribe to real-time updates (returns a stream)
#[tauri::command]
async fn subscribe_sensor_updates(window: Window) -> Result<(), String>;
```

Control-mode contract for Phase B:
- `set_fan_auto` keeps system-managed behavior as the default and fallback mode.
- Manual commands (`set_fan_speed`, sensor-linked control) must validate fan limits and safety thresholds before SMC write.
- App shutdown/crash path must restore all controllable fans to Auto mode.

### 4.2 Frontend Architecture (src/)

#### 4.2.1 Project Structure

```
src/
├── App.svelte              # Main app component
├── components/
│   ├── MenuBar.svelte      # Menu bar display
│   ├── FanDisplay.svelte   # Individual fan widget
│   ├── TempGraph.svelte    # Temperature chart
│   ├── TempDisplay.svelte  # Temperature widget
│   └── SettingsPanel.svelte
├── stores/
│   ├── sensorStore.ts      # Real-time sensor data (Svelte store)
│   └── settingsStore.ts    # User preferences
├── lib/
│   ├── tauriCommands.ts    # Tauri command wrappers
│   └── formatters.ts       # Data formatting utilities
└── types/
    └── index.ts            # TypeScript interfaces
```

#### 4.2.2 Data Flow

```
┌─────────────────────────────────────────────────────┐
│                   Sensor Store                       │
│  ┌──────────────────────────────────────────────┐  │
│  │  - Holds current sensor readings              │  │
│  │  - Subscribes to Tauri events                │  │
│  │  - Updates reactive Svelte components          │  │
│  └──────────────────────────────────────────────┘  │
└────────────────────┬────────────────────────────────┘
                     │ on update
                     ▼
┌─────────────────────────────────────────────────────┐
│                UI Components                         │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐  │
│  │ MenuBar    │  │ TempGraph  │  │ FanDisplay │  │
│  │ (reactive) │  │ (reactive) │  │ (reactive) │  │
│  └────────────┘  └────────────┘  └────────────┘  │
└─────────────────────────────────────────────────────┘
```

#### 4.2.3 Tauri Command Integration

```typescript
// src/lib/tauriCommands.ts
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export async function getFanSpeeds(): Promise<FanData[]> {
  return invoke('get_fan_speeds');
}

export async function getTemperatures(): Promise<TemperatureData> {
  return invoke('get_temperatures');
}

export function subscribeToUpdates(callback: (data: SensorData) => void) {
  return listen<SensorData>('sensor-update', (event) => {
    callback(event.payload);
  });
}
```

#### 4.2.4 Design System

The UI design system is intentionally lightweight and implementation-oriented:
- **Design tokens**: use shared variables for spacing, typography scale, radius, and semantic colors.
- **Thermal semantics**: all temperature displays share the same state palette (`normal`, `warm`, `hot`) so menu bar and window communicate risk consistently.
- **Mode affordances**: fan-control states (`Auto`, `Custom (Fixed)`, `Custom (Sensor-based)`) use distinct labels and visual emphasis to reduce accidental writes.
- **Theme parity**: every component supports macOS light/dark mode with the same information hierarchy and contrast targets.
- **Interaction consistency**: widget structure and labels mirror the monitoring table and control actions across views (tray quick view vs main window).

---

## 5. Data Flow Architecture

Data flow is mode-aware: Phase A is strictly read/broadcast, while Phase B adds a guarded command path for fan-control writes.

### 5.1 Real-Time Update Flow

```
┌─────────────┐     ┌──────────────────┐     ┌─────────────┐
│   Monitor   │────▶│  Tauri Event     │────▶│  Frontend   │
│   Service   │     │  (Broadcast)     │     │   Store     │
└─────────────┘     └──────────────────┘     └──────┬──────┘
     │                                               │
     │ poll every 1-3s                               │ notify
     ▼                                               ▼
┌─────────────┐                           ┌──────────────────┐
│     SMC     │                           │   UI Components  │
│   (macsmc)  │                           │   (Svelte)       │
└─────────────┘                           └──────────────────┘
```

### 5.2 Request/Response Flow (On-Demand)

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│   Frontend  │────▶│   Tauri      │────▶│   Rust      │
│  (invoke)   │     │   Command    │     │   Handler   │
└─────────────┘     └──────────────┘     └──────┬──────┘
     ▲                                          │
     │ response                                 │ query
     └──────────────────────────────────────────┘
                                          ┌─────────────┐
                                          │     SMC     │
                                          └─────────────┘
```

---

## 6. Security Considerations

### 6.1 Permission Requirements

| Operation | Permission Level | Notes |
|-----------|------------------|-------|
| Read temperatures | Standard user | No elevation needed |
| Read fan speeds | Standard user | No elevation needed |
| Set fan speeds | Elevated (root) | SMC write requires privileges |
| Auto-launch | Standard user | Via launchd or login items |

### 6.2 Thermal Safety

To prevent hardware damage:

1. **Minimum Fan Speed**: Enforce hardware minimum (typically 1200-2000 RPM)
2. **Emergency Override**: If CPU temp > 95°C, automatically return to system control
3. **Maximum Temperature Limits**: Prevent fan settings that would allow dangerous temps
4. **Safe Defaults**: Always start in "Auto" mode, require explicit user action for manual control
5. **Persistence**: Store last known-good settings; restore on app restart

### 6.3 Error Handling

```rust
pub enum FanControlError {
    SmcConnectionFailed,
    PermissionDenied,
    InvalidFanId,
    InvalidRpm { min: u32, max: u32, requested: u32 },
    ThermalSafetyTriggered { temp: f64, threshold: f64 },
    Timeout,
}
```

---

## 7. Apple Silicon vs Intel Compatibility

### 7.1 SMC Differences

| Aspect | Intel Macs | Apple Silicon (M1/M2/M3) |
|--------|------------|--------------------------|
| SMC Access | Via IOKit | Via IOKit (different keys) |
| Temperature Keys | TC0P, TC0D, etc. | Different naming convention |
| Fan Control | Well documented | Less documented, evolving |
| Compatibility | macsmc crate supports | macsmc crate supports |

### 7.2 Compatibility Strategy

1. **Use macsmc crate** - Handles both Intel and Apple Silicon internally
2. **Graceful degradation** - If a sensor is unavailable, show "N/A" instead of error
3. **Testing matrix** - Test on both Intel and M-series Macs
4. **Fallback polling** - If SMC read fails, retry with backoff

---

## 8. Alternatives Considered

### 8.1 Why Not Swift Native?

**Pros of Swift:**
- Best macOS integration
- Lowest resource usage possible
- Native system tray/menu bar APIs

**Cons of Swift:**
- Single platform (macOS only)
- Steeper learning curve for web developers
- Harder to integrate with modern UI libraries
- No built-in React/Vue/Svelte support

**Decision**: Tauri is close enough to native performance while offering better UI development experience.

### 8.2 Why Not Electron?

**Pros of Electron:**
- Mature ecosystem
- Large community
- Easy to find developers

**Cons of Electron:**
- High memory footprint (100MB+ baseline)
- Large bundle size
- Slower startup
- Higher CPU usage for simple apps

**Decision**: Tauri offers 10x smaller bundle and 10x lower memory usage.

### 8.3 Why Not Flutter?

**Pros of Flutter:**
- Cross-platform
- Beautiful UI
- Single codebase

**Cons of Flutter:**
- Heavy runtime
- Not truly native (custom rendering)
- Larger bundle size than Tauri
- macOS desktop is secondary priority for Flutter team

**Decision**: Tauri provides more native integration for macOS-specific features.

---

## 9. Implementation Roadmap

### Phase A: Read-Only Monitoring (Weeks 1-4)

| Week | Focus | Deliverables |
|------|-------|--------------|
| Week 1 | Project setup, Tauri scaffolding | Running app with menu bar |
| Week 2 | SMC integration, sensor reading | Temperature and fan data reading |
| Week 3 | UI development, real-time updates | Main window with graphs |
| Week 4 | Testing, optimization | <0.1% CPU usage, release v0.1.0 |

### Phase B: Fan Control (Weeks 5-8)

| Week | Focus | Deliverables |
|------|-------|--------------|
| Week 5 | Manual fan control | Set custom RPM |
| Week 6 | Fan curves | Temperature-based control |
| Week 7 | Profiles | Save/load configurations |
| Week 8 | Safety features, testing | Thermal protection, release v0.2.0 |

### Phase C: Polish (Weeks 9-12)

| Week | Focus | Deliverables |
|------|-------|--------------|
| Week 9 | Historical data | 24-hour graphs, data export |
| Week 10 | Alerts | Temperature notifications |
| Week 11 | Settings | Preferences panel |
| Week 12 | Documentation, release | v1.0.0 |

---

## 10. Key Dependencies

Dependency choices are driven by PRD constraints: low idle CPU usage, responsive UI updates, macOS-native integration, and safe control behavior.

### Rust (src-tauri/Cargo.toml)

```toml
[dependencies]
tauri = { version = "2.0", features = ["tray-icon"] }
macsmc = "0.1.7"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
```

### Frontend (package.json)

```json
{
  "dependencies": {
    "@tauri-apps/api": "^2.0",
    "chart.js": "^4.4",      // Temperature/history graphs
    "svelte": "^5.0"
  }
}
```

Tech stack rationale:
- **Tauri v2 + Rust**: native-feeling macOS app behavior with smaller footprint than Electron-class runtimes.
- **macsmc**: shared SMC access layer to support Intel and Apple Silicon paths with graceful degradation.
- **tokio**: efficient async polling/event scheduling for 1-3s update intervals.
- **Svelte + TypeScript**: reactive UI with low runtime overhead and explicit type contracts between frontend and backend.
- **Chart.js**: practical graphing for short-term thermal trend visualization.

---

## 11. Testing Strategy

### 11.1 Unit Tests (Rust)

- SMC interface mock tests
- Command handler tests
- Data formatting tests

### 11.2 Integration Tests

- End-to-end sensor reading on test Macs
- Menu bar functionality
- Window open/close behavior

### 11.3 Manual Testing Matrix

| Test Device | macOS Version | Intel/M-Series | Status |
|-------------|---------------|----------------|--------|
| MacBook Pro 16" | 14.x | M3 Pro | Required |
| MacBook Air | 14.x | M2 | Required |
| Mac mini | 13.x | M1 | Required |
| iMac | 13.x | Intel | Optional |

---

## 12. Getting Started (Technical)

### 12.1 Prerequisites

- macOS 12+ (Monterey or later)
- Rust toolchain (`rustup`, stable)
- Node.js 20+ and package manager (`npm`, `pnpm`, or `bun`)
- Xcode Command Line Tools

### 12.2 Initial Setup

```bash
# from repository root
npm install
```

### 12.3 Run in Development

```bash
# starts Vite + Tauri desktop app in dev mode
npm run tauri dev
```

### 12.4 Build for Local Validation

```bash
# production desktop build
npm run tauri build
```

### 12.5 Key Code Locations

- `src-tauri/src/smc.rs`: SMC read/write abstraction
- `src-tauri/src/monitor.rs`: polling loop and event broadcast
- `src-tauri/src/commands.rs`: Tauri command boundary
- `src/stores/sensorStore.ts`: reactive sensor state
- `src/lib/tauriCommands.ts`: frontend invoke/listen wrappers

### 12.6 Safety Notes for Developers

- Keep Phase A behavior read-only by default.
- During Phase B work, verify Auto restore behavior on quit/crash paths.
- Treat unavailable sensors and limited-control models as first-class cases (`N/A`, degraded UX, no panic paths).

---

## 13. References

1. mac-stats (Rust + Tauri reference): https://github.com/raro42/mac-stats
2. macsmc crate documentation: https://docs.rs/macsmc
3. Tauri v2 documentation: https://v2.tauri.app
4. macOS SMC documentation (Apple): https://developer.apple.com/documentation/iokit
5. Macs Fan Control (feature reference): https://github.com/crystalidea/macs-fan-control

---

*Document Version: 1.1*
*Last Updated: 2026-03-04*
*Status: Draft for Review*
