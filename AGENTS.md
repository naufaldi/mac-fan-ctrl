# AGENTS.md - mac-fan-ctrl

AI Agent context and conventions for the mac-fan-ctrl project.

## Project Overview

mac-fan-ctrl is a macOS fan control application built with **Tauri v2** (Rust backend + Svelte 5 frontend). It communicates with Apple's System Management Controller (SMC) to monitor temperatures and control fan speeds.

**Current State**: Hello-world vertical slice with basic Tauri bridge. SMC hardware integration is planned but not yet implemented.

## Technology Stack

| Layer | Technology | Version |
|-------|------------|---------|
| Frontend | Svelte 5 + TypeScript | 5.53.7 |
| Build Tool | Vite | 6.3.0 |
| Native Bridge | Tauri v2 | 2.10.2 |
| Backend | Rust | Edition 2021 |
| Package Manager | pnpm | workspace |
| Testing | Vitest + Playwright | 4.1.0-beta.5 |
| Linting | Biome | 2.4.5 |

## Directory Structure

```
mac-fan-ctrl/
├── src/                      # Frontend (Svelte/TS)
│   ├── App.svelte           # Root component
│   ├── main.ts              # Entry point
│   ├── lib/
│   │   └── tauriCommands.ts # API wrappers
│   └── __tests__/           # Unit tests
├── src-tauri/               # Backend (Rust)
│   ├── src/
│   │   ├── main.rs          # Entry point
│   │   └── commands.rs      # Tauri commands
│   ├── Cargo.toml          # Rust deps
│   └── tauri.conf.json     # Tauri config
├── docs/
│   ├── rfc.md              # Technical design
│   ├── prd.md              # Product requirements
│   ├── task.md             # Living ticketing doc
│   └── learning/           # Learning resources
├── e2e/                    # Playwright tests
├── pnpm-workspace.yaml     # pnpm workspace config
└── AGENTS.md               # This file
```

## Key Documentation

| Document | Purpose |
|----------|---------|
| `docs/github-workflow.md` | **GitHub Issues workflow** - How to create, track, and close issues |
| `docs/task.md` | Historical ticketing reference (new work uses GitHub Issues) |
| `docs/rfc.md` | Technical architecture, component diagrams, API design |
| `docs/prd.md` | Product requirements, Phase A/B features |
| `docs/learning/` | Rust, Tauri, SMC learning resources |

## Development Commands

```bash
# Development
pnpm dev              # Frontend only
pnpm tauri dev        # Full app with Rust

# Code quality
pnpm biome:check      # Lint
pnpm biome:format     # Format

# Testing
pnpm test             # Unit tests (Vitest)
pnpm playwright:test  # E2E fan-control regression (mocked Tauri)
pnpm fan-control:hardware-smoke  # Gated hardware checklist (FANGUARD_HARDWARE_SMOKE=1)
cd src-tauri && cargo test  # Rust tests

# Build
pnpm tauri build      # Release bundle
```

## RTK (Agent Shell Commands)

[rtk-ai/rtk](https://github.com/rtk-ai/rtk) compresses noisy dev command output before it reaches agent context. Prefer RTK-wrapped commands for regression runs and status checks; use raw commands when debugging a single failure and you need full logs.

Install (once per machine):

```bash
brew install rtk
# or: curl -fsSL https://raw.githubusercontent.com/rtk-ai/rtk/refs/heads/master/install.sh | sh
```

| Task | Human / CI | Agent (prefer RTK) |
|------|------------|---------------------|
| Unit tests (Vitest) | `pnpm test` | `rtk vitest run` or `rtk pnpm test` |
| Playwright E2E | `pnpm playwright:test` | `rtk pnpm playwright:test` |
| Rust tests | `cd src-tauri && cargo test` | `cd src-tauri && rtk cargo test` |
| Lint | `pnpm biome:check` | `rtk pnpm biome:check` |
| Git status / diff | `git status`, `git diff` | `rtk git status`, `rtk git diff` |

Fan-control regression (agent default):

```bash
rtk pnpm test
rtk pnpm playwright:test
cd src-tauri && rtk cargo test
rtk pnpm biome:check
```

Do **not** wrap:

- `pnpm dev`, `pnpm tauri dev` — interactive / streaming servers
- `FANGUARD_HARDWARE_SMOKE=1 pnpm fan-control:hardware-smoke` — real SMC writes; manual checklist only
- `sudo ...` — privilege elevation
- First investigation of an unknown failure — run the raw command once, then switch back to RTK for reruns

If RTK is installed, Cursor may also invoke it via `rtk hook cursor` in `~/.cursor/hooks.json` on Shell tool calls.

## Fan Control Verification

See [docs/fan-control-verification.md](docs/fan-control-verification.md) for the four-level verification strategy (Vitest/Rust unit tests, Playwright regression, optional agent-browser exploration, gated hardware smoke).

Default regression commands for fan-control changes:

```bash
pnpm test
pnpm playwright:test
cd src-tauri && cargo test
pnpm biome:check
```

Optional exploratory UI evidence (not CI):

```bash
pnpm dev
agent-browser open http://127.0.0.1:1420 && agent-browser snapshot -i
```

Gated real-hardware smoke (writes SMC fan settings — never run in CI):

```bash
FANGUARD_HARDWARE_SMOKE=1 pnpm fan-control:hardware-smoke
```

Native tray menus are AppKit UI; tray selection logic is tested in Rust (`fan_mode_menu_rows` in `src-tauri/src/tray.rs`). Playwright and agent-browser only drive the main webview.

## Coding Conventions

### TypeScript / Svelte

- Use Svelte 5 runes (`$state`, `$derived`, `$effect`) - no legacy `$:`
- Explicit return types on async functions: `async function(): Promise<T>`
- Use path aliases: `$lib/`, `$components/`, `$stores/`, `$types/`
- Styling must use Tailwind CSS v4 utility classes; do not add plain CSS styling in `src/app.css`
- Use a shared `cn()` helper (built with `clsx` + `tailwind-merge`) for conditional/overridable class names
- Prefer `cn(...)` over manual template-string class concatenation when classes are dynamic
- Error handling: `error instanceof Error` type guards
- Component testing: Mock Tauri with `vi.mock('@tauri-apps/api/core')`

### Rust

- Command naming: `snake_case` (e.g., `ping_backend`)
- Error handling: `Result<T, String>` for now, migrate to `thiserror` later
- Input validation at command entry points
- Unit tests inline in `#[cfg(test)]` modules
- Planned: `SmcError` enum with `thiserror` for structured errors

### Functional Programming (ALWAYS APPLY)

#### Pure Functions
- Functions must return same output for same input
- No mutations of parameters or global variables
- No side effects (I/O, mutations, external state)
- All data flows through parameters and return values

#### Immutability
- Use `const` exclusively, never `let` for mutable state
- Use spread syntax for object/array updates
- Never use mutating methods: `.push()`, `.pop()`, `.splice()`, delete operator
- Never reassign variables

#### No Loops
- Never use `for`, `while`, `do-while`
- Use `map`, `filter`, `reduce`, `flatMap`, `find`, `some`, `every`
- Chain array methods for data transformations
- Use recursion for iteration when necessary

#### Function Composition
- Functions must be small and single-purpose (max 15 lines)
- Compose functions with `compose()` or `pipe()`
- Avoid nested callbacks, use composition instead
- Each function does exactly one thing

#### Error Handling (Result Types)
- Use `Result<T, E>` type instead of throwing exceptions
- Never throw for expected errors
- Handle errors explicitly at call sites
- Use exhaustive pattern matching on Result variants

#### State Modeling (Discriminated Unions)
- Use tagged unions with `kind` or `status` discriminator
- Model all possible states explicitly
- Use exhaustive `switch` statements with no fallthrough
- Never use boolean flags for state

#### Type-Driven Development
- Define types before implementation
- Use branded types for domain primitives (Celsius, Rpm, FanId)
- Let compiler guide implementation via type errors
- Use discriminated unions over optional fields

#### Rust-Specific FP Rules
- Variables are immutable by default (`let`, not `let mut`)
- Use iterator chains: `.iter()`, `.filter()`, `.map()`, `.collect()`
- Use `Result<T, E>` and `Option<T>` for all fallible operations
- Use `?` operator for error propagation
- Pattern match on enums with exhaustive handling
- Never use `unwrap()` or `expect()` in production code

### Design & UI Guidelines (Brand Guide)

See [docs/design.md](docs/design.md) for the full parchment design system (ElevenLabs palette adapted through Apple HIG geometry).

- **Parchment + Apple HIG**: Warm paper surfaces (`#fdfcfc` canvas, `#f5f3f1` headers) with monochrome ink controls. Light-only — no dark mode.
- **Strict Native macOS UI**: Indistinguishable from a first-party macOS system utility (Activity Monitor, System Settings). Native window chrome stays OS-provided.
- **Layout**: Classic split-pane window (Fans left, Sensors right). Edge-to-edge hairline-divided panes — no floating web cards.
- **Controls**: HIG push buttons (7px radius), segmented controls for toggles (Auto/Custom), pop-up button for presets, HIG dialog panels (~10px).
- **Tables**: macOS table layouts with warm-sand headers, vertical column dividers, hairline row separators (no zebra striping).
- **Typography**: SF Pro Text (UI), SF Mono (data, tabular-nums). DM Sans 300 for display headings only.
- **Icons**: lucide-svelte outlined icons, monochrome (Driftwood/Ink) — no multicolor.
- **Colors**: Monochrome chrome. Ember `#ff4704` reserved exclusively for hot temperature (≥85°C) safety signals. No green/yellow/red semantic UI colors.
- **Styling**: Tailwind CSS v4 utility classes + shared `cn()` helper. Tokens live in `src/app.css` `:root` and `@theme`.

### Tauri Commands

```rust
// Backend
#[tauri::command]
pub fn ping_backend(message: String) -> Result<String, String> {
    if message.trim().is_empty() {
        return Err("message must not be empty".to_string());
    }
    Ok(format!("Hello from Rust: {message}"))
}
```

```typescript
// Frontend
import { invoke } from "@tauri-apps/api/core";

export async function pingBackend(message: string): Promise<string> {
    return invoke<string>("ping_backend", { message });
}
```

## GitHub Issues Workflow

This project uses GitHub Issues for task tracking. See [docs/github-workflow.md](docs/github-workflow.md) for the full workflow guide.

- Claude manages issues autonomously (create, close, comment)
- Use `gh issue list --state open` at session start
- Use `closes #N` in commit messages to auto-close issues
- Label issues with phase (`phase-a/b/c`) and area (`frontend`, `backend`, `smc`, `ui`)

## Definition of Done

From `docs/task.md` Section 5:

- Acceptance criteria implemented and verified
- Unit + integration tests (including failure path)
- Error and fallback strategy documented
- Rollback note included
- No known crashes introduced

## Phase Structure

- **Phase A** (Read-only monitoring): MACFAN-101.x stories
- **Phase B** (Fan control + safety): MACFAN-102.x stories
- **Phase C** (Polish + hardening): MACFAN-103.x stories

**Current Phase**: Sprint 0 (Foundation) - MACFAN-101.0 tasks

## Learning Resources

For contributors learning the stack:

| Resource | Topic |
|----------|-------|
| `docs/learning/rust-basics.md` | Rust fundamentals |
| `docs/learning/tauri-architecture.md` | Full stack overview |
| `docs/learning/macos-smc.md` | Hardware interface |
| `docs/learning/system-programming.md` | macOS IOKit |
| `docs/learning/dev-setup.md` | Environment setup |

## macOS Hardware APIs for Temperature & Sensor Data

When reading or extending sensor support, use the following APIs in priority order:

### 1. AppleSMC (already integrated)
- **What**: Apple's System Management Controller — the primary source for temperature, fan, and power sensors
- **How**: Via `macsmc` crate (IOKit FFI → `AppleSMC` kernel service)
- **Keys**: T* = temperature, F* = fans, P* = power
- **File**: `src-tauri/src/smc.rs`, `macsmc` crate
- **Limitation**: M1/M2/M3 expose fewer keys than Intel; some sensors (e.g. airflow TaLC/TaRC) have no physical hardware on Apple Silicon

### 2. IOHIDEventService / IOKit (already integrated)
- **What**: IOKit's HID event layer — used for sensors that Apple Silicon reports through the HID stack rather than SMC
- **How**: `ioreg -r -c IOHIDEventService -l` parsed in `src-tauri/src/apple_silicon_sensors.rs`
- **Gives**: Battery temps, GPU cluster, PMU die (fills gaps where SMC keys return 0 on M-series)
- **SensorSource**: `"iohid_iokit"`
- **hidutil**: `hidutil list` can enumerate HID services; useful for discovery

### 3. IOReport (private Apple framework — not yet integrated)
- **What**: Private IOKit sub-framework used internally by `powermetrics` and Activity Monitor
- **Gives on Apple Silicon**: Per-cluster CPU temps (E-core vs P-core), ANE (Neural Engine) temp, thermal pressure level, die-level granularity
- **How to access**: Requires reverse-engineered FFI bindings (`IOReport.h` is private). Projects like `vladkens/macmon` expose this via open-source Rust
- **Entitlement needed**: `com.apple.private.iokit.ioreporter` (sandboxed apps need this)
- **Reference**: https://github.com/vladkens/macmon

### 4. powermetrics CLI (system tool — not yet integrated)
- **What**: Apple's own thermal/power sampling tool, ships with macOS
- **Gives**: Thermal throttle reason, cluster temps, power draw, package temps
- **How**: `sudo powermetrics --samplers thermal,cpu_power -n 1 -f json`
- **Limitation**: Requires `sudo` — cannot run from a sandboxed Tauri app without special entitlements
- **Use case**: Throttle state detection, not continuous polling

### 5. sysctl (already used for topology, not temps)
- **What**: BSD kernel sysctl interface
- **Currently used for**: `hw.model`, `hw.perflevel0.physicalcpu`, `hw.perflevel1.physicalcpu` (CPU core counts)
- **Not useful for temps**: macOS does not expose temperature data through sysctl

### Sensor Source Priority
When a sensor key is available from multiple sources, prefer in this order:
1. `smc` — most authoritative for T* keys
2. `iohid_iokit` — fallback for Apple Silicon where SMC returns 0
3. `derived` — computed from other sensors (averages, sums)
4. `placeholder` — catalog entry with no hardware data (hidden from UI)

## Safety Considerations

When implementing Phase B (fan control):

- Always validate RPM bounds before SMC writes
- Check temperatures before allowing low fan speeds
- Implement emergency auto-restore on high temps
- Restore all fans to auto on app exit
- Write verification after SMC writes

## Testing Strategy

1. **Unit tests**: Mock Tauri/SMC for isolated testing
2. **Integration tests**: Test command handlers with real state
3. **E2E tests**: Playwright tests for critical user flows
4. **Hardware tests**: Manual validation on target Mac models

## Environment Requirements

- macOS (Intel or Apple Silicon)
- Node.js 18+
- Rust 1.77.2+
- pnpm 8+
- Xcode Command Line Tools

## Resources

- [Tauri Docs](https://tauri.app)
- [Svelte 5 Docs](https://svelte.dev/docs)
- [Rust Book](https://doc.rust-lang.org/book/)
- [IOKit Fundamentals](https://developer.apple.com/library/archive/documentation/DeviceDrivers/Conceptual/IOKitFundamentals/)
