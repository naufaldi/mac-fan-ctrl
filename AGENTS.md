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
| `docs/task.md` | **Primary ticketing document** - Check this first for active work |
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
pnpm playwright:test  # E2E tests
cd src-tauri && cargo test  # Rust tests

# Build
pnpm tauri build      # Release bundle
```

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

- **Strict Native macOS UI**: The application must look and feel indistinguishable from a first-party macOS system utility (like Activity Monitor) or high-quality native apps (like Macs Fan Control).
- **Layout**: Use a classic split-pane window layout (Fans on left, Sensors on right). Avoid web-centric "floating cards" or loose spacing. Use edge-to-edge tables.
- **Controls**: Mimic native macOS controls perfectly. Use native-looking Segmented Controls (connected buttons) for toggles, standard dropdowns, and native push buttons.
- **Tables**: Use standard macOS table layouts with gray headers, vertical column dividers, and alternating row background colors (zebra striping: `odd:` / `even:`).
- **Typography**: Strictly use system fonts (`SF Pro Text` for UI, `SF Mono` for data). Maintain high data density with small, crisp typography (e.g., 11px/12px for lists). Ensure tabular numbers (`font-variant-numeric: tabular-nums`) for all metrics.
- **Icons**: Use SF Symbols for sensor icons (e.g., wifi, battery.100, cpu) to maintain native consistency.
- **Colors**: Use semantic system colors that automatically adapt to light/dark mode, matching Apple's exact HIG hex values.

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

## Ticket Naming Convention

```
MACFAN-<epic>.<story>[-T<task>]

Examples:
- MACFAN-101.0     (Foundation epic)
- MACFAN-101.0-T1  (Repository tooling task)
- MACFAN-101.2     (Menu Bar + App Shell story)
- MACFAN-101.2-T1  (Backend telemetry task)
```

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
