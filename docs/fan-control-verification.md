# Fan Control Verification

Four verification levels keep fan-control work testable without relying on manual macOS testing alone.

| Level | Scope | Command / Tool |
|-------|--------|----------------|
| 1 — Unit/integration | Rust state, overlay, tray model, Vitest command/modal tests | `pnpm test`, `cd src-tauri && cargo test` |
| 2 — Playwright regression | Main-window Auto → Custom RPM → Auto with mocked Tauri | `pnpm playwright:test` |
| 3 — Exploratory (optional) | Snapshots/screenshots while debugging UI | `agent-browser open http://127.0.0.1:1420 && agent-browser snapshot -i` |
| 4 — Gated hardware smoke | Real SMC writes on macOS hardware (opt-in) | `FANGUARD_HARDWARE_SMOKE=1 pnpm fan-control:hardware-smoke` |

## Default regression commands

Run before claiming fan-control work is complete:

```bash
pnpm test
pnpm playwright:test
cd src-tauri && cargo test
cd src-tauri && cargo test --features helper-binary
pnpm biome:check
```

Agents with RTK installed should prefer the RTK-wrapped equivalents in [AGENTS.md](../AGENTS.md#rtk-agent-shell-commands).

## TestFlight verification

For App Store/TestFlight readiness, additionally run:

```bash
pnpm build:app-store
cd src-tauri && cargo test --features app-store
pnpm tauri:build:testflight
plutil -p src-tauri/Entitlements.testflight.plist
codesign -dvvv --entitlements :- src-tauri/target/release/bundle/macos/FanGuard.app
```

The TestFlight build must show `com.apple.security.app-sandbox = true`, must not bundle `fanguard-helper`, and must not expose fan write controls in the main window or tray.

## Playwright harness

Playwright starts Vite with `VITE_E2E_MOCK=true`, aliasing `@tauri-apps/api/*` to deterministic mocks in `src/e2e/`. This drives the main window only — not the native AppKit tray.

## Native tray

Tray mutual exclusivity is covered by Rust model tests in `src-tauri/src/tray.rs` (`fan_mode_menu_rows`). Validate the real tray menu during gated hardware smoke or manual release QA.

## Hardware smoke safety

- Only run with explicit approval and `FANGUARD_HARDWARE_SMOKE=1`
- Requires macOS, fans, and helper socket at `/var/run/fanguard.sock`
- Always restore fans to Auto before closing the app
- Never run hardware smoke in CI
