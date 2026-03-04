# Tauri Architecture in mac-fan-ctrl

Understanding how the frontend (Svelte), bridge (Tauri), and backend (Rust) work together.

## High-Level Flow

```
┌─────────────────┐     Tauri Commands      ┌─────────────────┐
│   Svelte UI     │  <──────────────────>  │   Rust Backend  │
│   (Frontend)    │     + Events            │   (src-tauri)   │
└─────────────────┘                         └─────────────────┘
        │                                           │
        │ DOM/APIs                            │ macOS SMC
        ▼                                           ▼
   WebView (WKWebView)                    Hardware Sensors
```

## Project Structure

```
mac-fan-ctrl/
├── src/                    # Frontend (Svelte + TypeScript)
│   ├── App.svelte         # Main app component
│   ├── lib/
│   │   ├── components/    # UI components
│   │   └── stores/       # Svelte stores for state
│   └── main.ts            # Entry point
├── src-tauri/             # Backend (Rust)
│   ├── src/
│   │   ├── main.rs        # Entry point, command handlers
│   │   ├── commands.rs    # Tauri command definitions
│   │   ├── smc.rs         # SMC hardware interface
│   │   └── monitor.rs     # Background monitoring service
│   ├── Cargo.toml         # Rust dependencies
│   └── tauri.conf.json    # Tauri configuration
├── docs/
└── package.json           # Node.js dependencies (pnpm)
```

## Communication Patterns

### 1. Commands (Frontend → Backend)

Frontend calls Rust functions directly:

```typescript
// src/lib/api.ts
import { invoke } from '@tauri-apps/api/core';

export async function getFanSpeeds(): Promise<FanInfo[]> {
    return await invoke('get_fan_speeds');
}

export async function setFanSpeed(fanId: number, rpm: number): Promise<void> {
    return await invoke('set_fan_speed', { fanId, rpm });
}
```

```rust
// src-tauri/src/commands.rs
use tauri::State;
use crate::AppState;

#[tauri::command]
pub fn get_fan_speeds(state: State<AppState>) -> Result<Vec<FanInfo>, String> {
    state.monitor_service
        .get_fans()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_fan_speed(
    fan_id: u8,
    rpm: u16,
    state: State<AppState>
) -> Result<(), String> {
    state.smc
        .set_fan_speed(fan_id, rpm)
        .map_err(|e| e.to_string())
}
```

Register commands in main.rs:

```rust
// src-tauri/src/main.rs
fn main() {
    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::get_fan_speeds,
            commands::set_fan_speed,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 2. Events (Backend → Frontend)

Rust can emit events that Svelte listens to:

```rust
// src-tauri/src/monitor.rs
use tauri::{AppHandle, Emitter};

pub fn broadcast_temps(app: &AppHandle, temps: &TemperatureData) {
    app.emit("temperature-update", temps)
        .expect("failed to emit event");
}
```

```typescript
// src/lib/stores/temperature.ts
import { listen } from '@tauri-apps/api/event';
import { writable } from 'svelte/store';

export const temperatures = writable<TemperatureData>({});

// Listen for backend events
listen('temperature-update', (event) => {
    temperatures.set(event.payload);
});
```

### 3. State Management

Shared state between commands:

```rust
// src-tauri/src/state.rs
use std::sync::{Arc, Mutex};

pub struct AppState {
    pub smc: Arc<Mutex<SmcInterface>>,
    pub monitor: Arc<Mutex<MonitorService>>,
    pub config: Arc<Mutex<AppConfig>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            smc: Arc::new(Mutex::new(SmcInterface::new())),
            monitor: Arc::new(Mutex::new(MonitorService::new())),
            config: Arc::new(Mutex::new(AppConfig::default())),
        }
    }
}
```

Tauri manages the state lifetime:

```rust
// main.rs
.manage(AppState::new())  // Created at startup, accessible via State<T>
```

## Frontend Architecture (Svelte 5)

### Reactive State with Runes

```svelte
<!-- src/App.svelte -->
<script lang="ts">
  // Svelte 5 runes for reactivity
  let fanSpeeds = $state<FanInfo[]>([]);
  let selectedFan = $state<number | null>(null);
  
  // Derived state
  let totalRpm = $derived(
    fanSpeeds.reduce((sum, f) => sum + f.current_rpm, 0)
  );
  
  // Effects for side actions
  $effect(() => {
    if (fanSpeeds.length > 0) {
      console.log('Fans updated:', fanSpeeds);
    }
  });
</script>

<div class="fan-display">
  <h1>Total RPM: {totalRpm}</h1>
  {#each fanSpeeds as fan}
    <FanCard {fan} />
  {/each}
</div>
```

### Store for Global State

```typescript
// src/lib/stores/fans.ts
import { writable } from 'svelte/store';
import { getFanSpeeds } from '../api';

function createFanStore() {
    const { subscribe, set, update } = writable<FanInfo[]>([]);
    
    return {
        subscribe,
        refresh: async () => {
            const fans = await getFanSpeeds();
            set(fans);
        },
        updateFan: (id: number, updates: Partial<FanInfo>) => {
            update(fans => fans.map(f => 
                f.id === id ? { ...f, ...updates } : f
            ));
        }
    };
}

export const fanStore = createFanStore();
```

## Backend Architecture (Rust)

### Layered Design

```
┌─────────────────────────────────────┐
│  Commands (Tauri handlers)           │
│  - JSON serialization                │
│  - Error to string conversion        │
├─────────────────────────────────────┤
│  Services (Business logic)           │
│  - MonitorService: polling, state    │
│  - ProfileService: curve calculations│
├─────────────────────────────────────┤
│  Adapters (Hardware abstraction)     │
│  - SmcInterface: SMC key reading     │
│  - FanController: Safe write ops     │
├─────────────────────────────────────┤
│  macOS SMC (System Framework)        │
└─────────────────────────────────────┘
```

### Safety Layer for Hardware Access

```rust
// src-tauri/src/safety.rs
pub struct SafeFanController {
    smc: SmcInterface,
    max_rpm: u16,
    critical_temp: f64,
}

impl SafeFanController {
    pub fn set_speed(&self, fan_id: u8, target_rpm: u16) -> Result<(), FanError> {
        // Safety checks before hardware access
        if target_rpm > self.max_rpm {
            return Err(FanError::RpmTooHigh);
        }
        
        let current_temp = self.smc.read_cpu_temp()?;
        if current_temp > self.critical_temp && target_rpm < 3000 {
            return Err(FanError::WouldOverheat);
        }
        
        // Safe to write to hardware
        self.smc.set_fan_target(fan_id, target_rpm)
    }
    
    pub fn emergency_shutdown(&self) -> Result<(), FanError> {
        // Return all fans to auto mode
        for fan_id in 0..self.smc.fan_count() {
            self.smc.set_fan_auto(fan_id)?;
        }
        Ok(())
    }
}
```

## Build Process

```
pnpm dev
    ├── Vite dev server (port 5173)
    └── Tauri runs WebView pointing to Vite

pnpm tauri build
    ├── Vite builds frontend to dist/
    └── Rust embeds dist/ and compiles binary
        └── Output: src-tauri/target/release/mac-fan-ctrl
```

## Security Considerations

### Permissions (tauri.conf.json)

```json
{
  "permissions": [
    {
      "identifier": "allow-execute",
      "allow": [{"cmd": "smc", "args": true}]
    }
  ]
}
```

### Sandboxing

- Frontend runs in WKWebView with limited API access
- All system access goes through explicit Rust commands
- SMC write operations require elevated permissions

## Development Workflow

```bash
# 1. Install dependencies
pnpm install
cd src-tauri && cargo fetch

# 2. Run in development mode
pnpm tauri dev
# - Vite hot reloads frontend
# - Rust recompiles on changes (slower)

# 3. Test the interface
pnpm test           # Frontend unit tests
cd src-tauri && cargo test  # Backend tests

# 4. Build release
pnpm tauri build    # Creates .app bundle
```

## Debugging

### Frontend (Svelte/Vite)

- Browser DevTools (Cmd+Option+I in app)
- Console logs from `console.log()`
- Vite error overlay

### Backend (Rust)

```rust
// Add logging
log::info!("Fan speed updated: {} -> {} RPM", fan_id, rpm);
log::debug!("SMC key read: {} = {}", key, value);
```

Run with log level:
```bash
RUST_LOG=debug pnpm tauri dev
```

## Next Steps

- [Rust Basics](./rust-basics.md) - Learn the backend language
- [Tauri Commands](./tauri-commands.md) - Deeper dive into IPC
- [macOS SMC](./macos-smc.md) - Hardware interface
- [Testing Strategy](./testing.md) - Testing all layers
