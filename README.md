# FanGuard

A native macOS fan control utility built with Tauri v2 (Rust backend) and Svelte 5 (frontend). Monitor temperatures and manage fan speeds through Apple's System Management Controller (SMC).

> **Beta** — This app is in active development. Please report issues on [GitHub](https://github.com/naufaldi/mac-fan-ctrl/issues).

## Features

- Real-time temperature monitoring via SMC and IOKit sensors
- Per-fan speed control: automatic, constant RPM, or sensor-based curves
- Menu bar tray with live CPU temperature display
- Custom preset save/restore
- Emergency thermal override safety system
- Light and dark mode (follows macOS system appearance)

## Requirements

- macOS 13 Ventura or later (Intel or Apple Silicon)
- Fan speed control requires running as root — the app will prompt on first use

## Installation

### Direct Download

Download the latest `.dmg` from [Releases](https://github.com/naufaldi/mac-fan-ctrl/releases).

### Homebrew

```bash
brew tap naufaldi/tap
brew install --cask fanguard
```

## Development

```bash
# Install dependencies
pnpm install

# Run full app (frontend + Rust backend)
pnpm tauri dev

# Run with fan control (requires root)
sudo pnpm tauri dev

# Frontend unit tests
pnpm test

# Rust unit tests
cd src-tauri && cargo test

# Lint
pnpm biome:check
```

### Prerequisites

- Node.js 18+
- pnpm 8+
- Rust 1.77.2+
- Xcode Command Line Tools

## License

MIT
