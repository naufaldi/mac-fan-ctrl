# mac-fan-ctrl Learning Center

Welcome to the learning resources for mac-fan-ctrl. This project is designed as a learning opportunity for backend and system programming, specifically using Rust, Tauri, and macOS system APIs.

## Learning Roadmap

### Phase 1: Foundation (Start Here)
1. [Project Architecture](./tauri-architecture.md) - Understanding the Tauri + Rust + Svelte stack
2. [Development Setup](./dev-setup.md) - Local development with pnpm and Cargo
3. [TypeScript to Rust](./ts-to-rust.md) - Mapping concepts for web developers

### Phase 2: Rust Fundamentals
4. [Rust Basics](./rust-basics.md) - Core concepts for this project
5. [Rust Ownership & Borrowing](./rust-ownership.md) - Memory safety in practice
6. [Error Handling in Rust](./rust-errors.md) - The Result/Option pattern

### Phase 3: System Programming
7. [macOS SMC](./macos-smc.md) - System Management Controller basics
8. [System Programming Concepts](./system-programming.md) - Low-level macOS programming
9. [Unsafe Rust](./unsafe-rust.md) - When and how to use unsafe code

### Phase 4: Integration
10. [Tauri Commands](./tauri-commands.md) - Frontend-Backend communication
11. [Async Rust](./async-rust.md) - Concurrency with tokio
12. [Testing Strategy](./testing.md) - Unit, integration, and E2E tests

## Quick Reference

### Technology Stack

| Layer | Technology | Purpose | Learning Resource |
|-------|------------|---------|-------------------|
| Frontend | Svelte 5 + TypeScript | UI components, reactivity | [Svelte Docs](https://svelte.dev/docs) |
| Bridge | Tauri v2 | Native app shell, IPC | [Tauri Docs](https://tauri.app) |
| Backend | Rust | System access, performance | [Rust Book](https://doc.rust-lang.org/book/) |
| System | macOS SMC | Hardware sensors, fan control | [macos-smc docs](./macos-smc.md) |

### Common Commands

```bash
# Frontend development
pnpm dev              # Start Vite dev server
pnpm build            # Build for production
pnpm test             # Run Vitest tests
pnpm playwright:test  # Run E2E tests

# Backend development
cd src-tauri
cargo build           # Build Rust backend
cargo test            # Run Rust tests
cargo clippy          # Lint Rust code

# Tauri (both frontend + backend)
pnpm tauri dev        # Run Tauri app in dev mode
pnpm tauri build      # Build release binary
```

## Learning by Doing

Each PRD story in this project is designed to teach specific concepts:

- **US-A1 (Menu Bar)**: Tauri system tray, window management
- **US-A2 (Temperature Monitoring)**: Async Rust, SMC reading, event streaming
- **US-A3 (Performance)**: Rust optimization, resource management
- **US-A4 (Multi-Fan)**: Data structures, error handling
- **US-B1 (Manual Controls)**: Permission handling, safe system writes
- **US-B2 (Sensor Curves)**: Algorithms, state machines
- **US-B3 (Profiles)**: Persistence, serialization
- **US-B4 (Safety)**: Guards, fallbacks, defensive programming

## Resources

### Official Documentation
- [The Rust Programming Language](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Tauri Documentation](https://tauri.app/v1/guides/)
- [Svelte Documentation](https://svelte.dev/docs)
- [macOS System Management Controller](https://developer.apple.com/library/archive/technotes/tn2169/_index.html)

### Recommended Tools
- **Rust**: rust-analyzer (VS Code/Cursor extension)
- **Tauri**: Tauri CLI (`cargo install tauri-cli`)
- **TypeScript**: TypeScript compiler and IDE support
- **Testing**: Vitest (frontend), Cargo test (backend), Playwright (E2E)

### Community
- [Rust Discord](https://discord.gg/rust-lang)
- [Tauri Discord](https://discord.gg/tauri)
- [r/rust](https://reddit.com/r/rust)
- [r/MacOSProgramming](https://reddit.com/r/MacOSProgramming)

## Contributing to Learning Docs

Found something confusing? Want to add a tutorial? PRs to the `docs/learning/` directory are welcome! Follow the existing format and include practical examples from the mac-fan-ctrl codebase.
