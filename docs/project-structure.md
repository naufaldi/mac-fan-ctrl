# mac-fan-ctrl Project Structure

This document outlines the complete directory and file organization for the mac-fan-ctrl application.

---

## 1. Root Directory Structure

```
mac-fan-ctrl/
├── docs/                        # Documentation
│   ├── prd.md                  # Product Requirements Document
│   ├── rfc.md                  # Request for Comments (Technical Design)
│   ├── project-structure.md    # This file
│   └── diagrams/
│       └── architecture.md     # Mermaid architecture diagrams
│
├── src/                        # Frontend source code (Svelte/TypeScript)
│   ├── App.svelte             # Root Svelte component
│   ├── main.ts                # Frontend entry point
│   ├── components/            # UI components
│   ├── stores/                # Svelte stores (state management)
│   ├── lib/                   # Utility libraries
│   └── types/                 # TypeScript type definitions
│
├── src-tauri/                 # Rust backend (Tauri)
│   ├── Cargo.toml             # Rust dependencies
│   ├── build.rs               # Build script
│   ├── src/                   # Rust source code
│   ├── icons/                 # Application icons
│   └── capabilities/          # Tauri capability configs
│
├── public/                    # Static assets (frontend)
├── package.json               # Node.js dependencies
├── tsconfig.json              # TypeScript configuration
├── vite.config.ts             # Vite build configuration
├── svelte.config.js           # Svelte configuration
├── tailwind.config.js         # Tailwind CSS configuration
├── index.html                 # HTML entry point
└── README.md                  # Project readme
```

---

## 2. Frontend Source (`src/`)

### 2.1 Components Directory (`src/components/`)

```
src/components/
├── MenuBar.svelte              # System tray menu bar display
├── MainWindow.svelte           # Main application window
├── FanDisplay.svelte           # Individual fan speed widget
├── TempDisplay.svelte          # Temperature sensor widget
├── TempGraph.svelte            # Temperature history chart
├── SettingsPanel.svelte        # Settings/configuration UI
├── SensorGrid.svelte           # Grid layout for multiple sensors
└── common/
    ├── Button.svelte           # Reusable button component
    ├── Card.svelte             # Card container component
    ├── Tooltip.svelte          # Tooltip wrapper
    └── Modal.svelte            # Modal dialog component
```

### 2.2 Stores Directory (`src/stores/`)

```
src/stores/
├── sensorStore.ts              # Real-time sensor data store
├── settingsStore.ts            # User preferences store
└── uiStore.ts                  # UI state store (window open, theme)
```

### 2.3 Library Directory (`src/lib/`)

```
src/lib/
├── tauriCommands.ts            # Tauri command invocations
├── formatters.ts               # Data formatting utilities
├── validators.ts               # Input validation functions
└── constants.ts                # Application constants
```

### 2.4 Types Directory (`src/types/`)

```
src/types/
└── index.ts                    # TypeScript interfaces and types
```

**Type Definitions:**

```typescript
// Sensor data types
export interface FanData {
  id: number;
  name: string;
  currentRpm: number;
  targetRpm?: number;
  maxRpm: number;
}

export interface CpuTemperatures {
  package: number;
  cores: number[];
}

export interface TemperatureData {
  cpu: CpuTemperatures;
  gpu?: number;
  battery?: number;
  ssd?: number;
}

export interface SensorData {
  fans: FanData[];
  temperatures: TemperatureData;
  timestamp: number;
}

// Settings types
export interface FanCurve {
  name: string;
  points: { temp: number; speed: number }[];
}

export interface AppSettings {
  updateInterval: number;
  temperatureUnit: 'celsius' | 'fahrenheit';
  showMenuBarIcon: boolean;
  startAtLogin: boolean;
  activeProfile: string;
  profiles: FanProfile[];
}

export interface FanProfile {
  id: string;
  name: string;
  curve: FanCurve;
}
```

---

## 3. Rust Backend (`src-tauri/src/`)

### 3.1 Source Files

```
src-tauri/src/
├── main.rs                     # Application entry point
├── lib.rs                      # Library exports (optional)
├── commands.rs                 # Tauri command handlers
├── smc.rs                      # SMC interface module
├── monitor.rs                  # Background monitoring service
├── error.rs                    # Error types and handling
├── state.rs                    # Application state management
└── tray.rs                     # System tray menu handlers
```

### 3.2 File Responsibilities

| File | Responsibility |
|------|----------------|
| `main.rs` | App initialization, Tauri builder setup, system tray configuration |
| `commands.rs` | Tauri command handlers exposed to frontend |
| `smc.rs` | SMC (System Management Controller) communication via macsmc crate |
| `monitor.rs` | Background polling service for continuous monitoring |
| `error.rs` | Custom error types using `thiserror` |
| `state.rs` | Tauri managed state (SmcClient, Config, MonitorService) |
| `tray.rs` | System tray menu builders and event handlers |

### 3.3 Example: main.rs Structure

```rust
// src-tauri/src/main.rs

mod commands;
mod error;
mod monitor;
mod smc;
mod state;
mod tray;

use state::AppState;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .manage(AppState::new())
        .setup(|app| {
            // Initialize system tray
            tray::setup_tray(app)?;
            // Start background monitor
            monitor::start_service(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_fan_speeds,
            commands::get_temperatures,
            commands::set_fan_speed,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 3.4 Example: commands.rs Structure

```rust
// src-tauri/src/commands.rs

use crate::error::Result;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn get_fan_speeds(state: State<'_, AppState>) -> Result<Vec<FanData>> {
    let smc = state.smc.lock().await;
    smc.read_fan_speeds()
}

#[tauri::command]
pub async fn get_temperatures(state: State<'_, AppState>) -> Result<TemperatureData> {
    let smc = state.smc.lock().await;
    smc.read_temperatures()
}

#[tauri::command]
pub async fn set_fan_speed(
    state: State<'_, AppState>,
    fan_id: u8,
    rpm: u32,
) -> Result<()> {
    let mut smc = state.smc.lock().await;
    smc.set_fan_speed(fan_id, rpm)
}
```

---

## 4. Configuration Files

### 4.1 Rust Configuration

**`src-tauri/Cargo.toml`**:
```toml
[package]
name = "mac-fan-ctrl"
version = "0.1.0"
edition = "2021"

[dependencies]
tauri = { version = "2.0", features = ["tray-icon", "unstable"] }
macsmc = "0.1.7"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"

[features]
default = ["custom-protocol"]
custom-protocol = ["tauri/custom-protocol"]
```

### 4.2 Frontend Configuration

**`package.json`**:
```json
{
  "name": "mac-fan-ctrl",
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview",
    "tauri": "tauri"
  },
  "dependencies": {
    "@tauri-apps/api": "^2.0.0",
    "chart.js": "^4.4.0",
    "chartjs-adapter-date-fns": "^3.0.0"
  },
  "devDependencies": {
    "@sveltejs/vite-plugin-svelte": "^4.0.0",
    "@tauri-apps/cli": "^2.0.0",
    "svelte": "^5.0.0",
    "svelte-check": "^4.0.0",
    "typescript": "^5.0.0",
    "vite": "^5.0.0",
    "tailwindcss": "^3.4.0"
  }
}
```

**`tsconfig.json`**:
```json
{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "module": "ESNext",
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true
  },
  "include": ["src"]
}
```

---

## 5. Assets and Resources

### 5.1 Icons (`src-tauri/icons/`)

```
src-tauri/icons/
├── 32x32.png                   # Small icon
├── 128x128.png                 # Standard icon
├── 128x128@2x.png              # Retina icon
├── icon.icns                   # macOS icon set
└── icon.ico                    # Windows icon (if cross-platform)
```

### 5.2 Static Assets (`public/`)

```
public/
└── (empty for now - icons managed by Tauri)
```

---

## 6. Build and Distribution

### 6.1 Build Outputs

```
src-tauri/
├── target/
│   ├── debug/                  # Debug builds
│   └── release/                # Release builds
│       └── mac-fan-ctrl        # Unix executable
│
├── gen/                        # Tauri generated files
│
└── target/release/bundle/      # Bundled applications
    ├── dmg/
    │   └── mac-fan-ctrl.dmg   # macOS disk image
    └── macos/
        └── mac-fan-ctrl.app   # macOS application bundle
```

### 6.2 Development Workflow

**Development commands:**

```bash
# Start development server with hot reload
cargo tauri dev

# Build for production
cargo tauri build

# Build release with bundle
cargo tauri build --release

# Run only frontend dev server
npm run dev

# Build only frontend
npm run build
```

---

## 7. Testing Structure

### 7.1 Rust Tests

```
src-tauri/src/
├── smc.rs
└── smc_test.rs                 # Unit tests for SMC module

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fan_data_parsing() {
        // Test cases
    }
}
```

### 7.2 Frontend Tests

```
src/
├── components/
│   └── FanDisplay.svelte
└── components/__tests__/
    └── FanDisplay.test.ts      # Component tests
```

---

## 8. Documentation Standards

### 8.1 Code Documentation

**Rust:**
```rust
/// Reads current fan speeds from SMC
/// 
/// # Returns
/// 
/// A vector of `FanData` structs containing fan information
/// 
/// # Errors
/// 
/// Returns `SmcError` if SMC communication fails
pub fn read_fan_speeds(&self) -> Result<Vec<FanData>, SmcError> {
    // Implementation
}
```

**TypeScript:**
```typescript
/**
 * Formats RPM value for display
 * @param rpm - Raw RPM value from SMC
 * @returns Formatted string (e.g., "2,450 RPM")
 */
export function formatRpm(rpm: number): string {
  return `${rpm.toLocaleString()} RPM`;
}
```

---

## 9. Naming Conventions

### 9.1 Rust

- **Files**: `snake_case.rs` (e.g., `fan_control.rs`)
- **Types**: `PascalCase` (e.g., `FanData`, `TemperatureReading`)
- **Functions**: `snake_case` (e.g., `read_temperatures()`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `MAX_FAN_SPEED`)
- **Modules**: `snake_case` (e.g., `mod smc_interface`)

### 9.2 TypeScript/Svelte

- **Files**: `PascalCase.svelte` for components, `camelCase.ts` for utilities
- **Interfaces**: `PascalCase` (e.g., `interface FanData`)
- **Functions**: `camelCase` (e.g., `getFanSpeeds()`)
- **Constants**: `SCREAMING_SNAKE_CASE` or `camelCase` for local
- **Stores**: `camelCaseStore` (e.g., `sensorStore`, `settingsStore`)

---

*Document Version: 1.0*
*Last Updated: 2026-03-02*
