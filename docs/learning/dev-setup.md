# Development Setup Guide

Setting up your environment for mac-fan-ctrl development with pnpm and Cargo.

## Prerequisites

### Required Software

| Tool | Version | Purpose | Install Command |
|------|---------|---------|-----------------|
| Node.js | 18+ | Frontend runtime | `brew install node` |
| pnpm | 8+ | Package manager | `brew install pnpm` |
| Rust | 1.70+ | Backend language | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Xcode CLI | latest | macOS headers | `xcode-select --install` |

### Verify Installation

```bash
# Check versions
node --version    # v18.x or higher
pnpm --version    # 8.x or higher
cargo --version   # 1.70 or higher

# Check macOS headers
xcrun --show-sdk-path
```

## Project Setup

### 1. Clone and Install

```bash
# Clone the repository
git clone https://github.com/your-org/mac-fan-ctrl.git
cd mac-fan-ctrl

# Install frontend dependencies
pnpm install

# Install Rust dependencies
cd src-tauri
cargo fetch
cd ..
```

### 2. Verify Setup

```bash
# Check pnpm workspace
pnpm list

# Verify Tauri CLI
cargo tauri --version
```

## Development Workflow

### Frontend-Only Development

For UI work without needing the Rust backend:

```bash
# Start Vite dev server only
pnpm dev

# Access at http://localhost:5173
# Note: Tauri commands will fail (mock data mode)
```

### Full Stack Development

For features requiring Rust backend:

```bash
# Start Tauri (includes Vite dev server)
pnpm tauri dev

# Application window opens automatically
# Rust code hot-reloads (slower than Vite)
```

### Building for Release

```bash
# Create production build
pnpm tauri build

# Output locations:
# - src-tauri/target/release/bundle/dmg/*.dmg
# - src-tauri/target/release/bundle/macos/*.app
```

## pnpm Workspace Commands

### Package Management

```bash
# Add dependency to frontend
pnpm add <package>

# Add dev dependency
pnpm add -D <package>

# Add to specific workspace (if we add more)
pnpm add <package> --filter <workspace-name>

# Update dependencies
pnpm update

# Clean install
rm -rf node_modules pnpm-lock.yaml
pnpm install
```

### Running Scripts

```bash
# Frontend development
pnpm dev              # Vite dev server
pnpm build            # Production build
pnpm preview          # Preview production build

# Code quality
pnpm biome:check      # Linting
pnpm biome:format     # Formatting
pnpm check            # Type checking (svelte-check)

# Testing
pnpm test             # Unit tests (Vitest)
pnpm test:watch       # Watch mode
pnpm test:coverage    # With coverage report
pnpm playwright:test  # E2E tests

# Tauri
pnpm tauri dev        # Development mode
pnpm tauri build      # Release build
pnpm tauri icon       # Generate app icons
```

## Rust Development

### Cargo Commands

```bash
cd src-tauri

# Building
cargo build              # Debug build
cargo build --release    # Optimized build
cargo check              # Fast syntax/type check
cargo clippy             # Linting
cargo fmt                # Format code

# Testing
cargo test               # Run all tests
cargo test <test_name>   # Run specific test
cargo test -- --nocapture  # Show println output

# Dependencies
cargo add <crate>        # Add dependency
cargo update             # Update dependencies
cargo tree               # View dependency tree

# Documentation
cargo doc                # Generate docs
cargo doc --open         # Generate and open
```

### Rust in Tauri Context

```bash
# Run with logging
RUST_LOG=debug pnpm tauri dev

# Run with trace-level logging
RUST_LOG=trace pnpm tauri dev

# Filter logs
RUST_LOG=mac_fan_ctrl=debug,info pnpm tauri dev
```

## IDE Setup

### Cursor / VS Code

Recommended extensions:

```json
// .vscode/extensions.json
{
  "recommendations": [
    "rust-lang.rust-analyzer",      // Rust support
    "tauri-apps.tauri-vscode",        // Tauri helpers
    "svelte.svelte-vscode",          // Svelte support
    "biomejs.biome",                  // Linting/Formatting
    "dbaeumer.vscode-eslint",         // TypeScript linting
    "bradlc.vscode-tailwindcss",      // Tailwind CSS
  ]
}
```

Settings:

```json
// .vscode/settings.json
{
  "rust-analyzer.cargo.features": ["tauri"],
  "rust-analyzer.checkOnSave.command": "clippy",
  "editor.formatOnSave": true,
  "editor.defaultFormatter": "biomejs.biome",
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  }
}
```

## Testing Strategy

### Unit Tests (Rust)

```bash
cd src-tauri

# Run all tests
cargo test

# Run with output visible
cargo test -- --nocapture

# Run specific test
cargo test test_fan_reading

# Run ignored tests (including integration tests)
cargo test -- --ignored
```

### Unit Tests (Frontend)

```bash
# Run Vitest
pnpm test

# Watch mode
pnpm test:watch

# With UI
pnpm vitest --ui

# Coverage
pnpm test:coverage
```

### E2E Tests

```bash
# Install Playwright browsers (first time)
pnpm playwright install

# Run E2E tests
pnpm playwright:test

# Run specific test
pnpm playwright test hello-world

# Debug mode
pnpm playwright test --debug

# UI mode
pnpm playwright test --ui
```

## Debugging

### Frontend Debugging

1. **Browser DevTools**: In Tauri window, press `Cmd+Option+I`
2. **Console Logging**: Use `console.log()` in Svelte/TypeScript
3. **Vite HMR**: Changes reflect immediately in dev mode

### Rust Debugging

1. **Logging**:
```rust
log::info!("Fan speed: {}", rpm);
log::debug!("SMC read: {:?}", value);
```

2. **Run with logs**:
```bash
RUST_LOG=debug pnpm tauri dev
```

3. **LLDB/GDB** (for complex issues):
```bash
# Build debug version
cd src-tauri
cargo build

# Run with debugger
lldb target/debug/mac-fan-ctrl
```

## Common Issues

### Permission Denied (SMC)

```bash
# SMC writes require elevation
# The app will prompt for admin access
# Or run with sudo (not recommended for dev)
```

### Rust Build Failures

```bash
# Clean build
cd src-tauri
cargo clean
cargo build

# Update dependencies
cargo update
```

### Node/Module Issues

```bash
# Clear pnpm cache
pnpm store prune

# Reinstall dependencies
rm -rf node_modules
pnpm install
```

### Xcode / SDK Issues

```bash
# Reinstall Xcode CLI tools
sudo rm -rf /Library/Developer/CommandLineTools
xcode-select --install

# Set SDK path if needed
export SDKROOT=$(xcrun --show-sdk-path)
```

## Project Structure Reference

```
mac-fan-ctrl/
├── src/                      # Frontend (Svelte/TS)
│   ├── lib/
│   │   ├── components/      # Svelte UI components
│   │   ├── stores/         # Svelte stores
│   │   └── api.ts          # Tauri command wrappers
│   ├── App.svelte          # Main app component
│   └── main.ts             # Frontend entry
├── src-tauri/               # Backend (Rust)
│   ├── src/
│   │   ├── main.rs         # Entry point
│   │   ├── commands.rs     # Tauri command handlers
│   │   ├── smc.rs          # SMC interface
│   │   └── monitor.rs      # Background monitoring
│   ├── Cargo.toml         # Rust dependencies
│   └── tauri.conf.json    # Tauri configuration
├── docs/
│   ├── rfc.md              # Technical design
│   ├── prd.md              # Product requirements
│   ├── task.md             # Ticketing & planning
│   └── learning/           # Learning resources
├── e2e/                    # Playwright E2E tests
├── package.json            # Node dependencies
├── pnpm-workspace.yaml     # pnpm workspace config
└── vite.config.ts          # Vite configuration
```

## Git Workflow

```bash
# Create feature branch
git checkout -b feature/MACFAN-101.2-T1-tray-ui

# Make changes, commit with ticket reference
git commit -m "MACFAN-101.2-T1: Add menu bar tray icon

- Implement system tray with Tauri API
- Add show/hide window on click
- Include status icon based on temps"

# Push and create PR
git push -u origin feature/MACFAN-101.2-T1-tray-ui
```

## Performance Profiling

### Rust Profiling

```bash
# Build with debug symbols
cd src-tauri
cargo build --profile=release-with-debug

# Run profiler
# On macOS: Instruments, samply, or cargo-flamegraph
```

### Frontend Profiling

- Chrome DevTools Performance tab
- Svelte DevTools browser extension
- Vite build analysis: `pnpm build --analyze`

## Next Steps

- [Rust Basics](./rust-basics.md) - Learn Rust fundamentals
- [Tauri Architecture](./tauri-architecture.md) - Understand the stack
- [Testing Strategy](./testing.md) - Deep dive into testing
