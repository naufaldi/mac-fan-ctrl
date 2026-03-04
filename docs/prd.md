# Product Requirements Document: mac-fan-ctrl

## 1. Overview

### 1.1 Project Description

mac-fan-ctrl is a lightweight macOS desktop application for monitoring and controlling Mac fan speeds. Built with Rust and Tauri, it provides real-time temperature and fan speed monitoring through a native menu bar interface with minimal system resource usage.

Product direction: align core user workflows with Macs Fan Control on macOS (monitoring table, Auto/Custom control, presets, safe restore behavior).

### 1.2 Goals

1. **Primary Goal**: Provide Mac users with visibility into their system's thermal state and fan performance
2. **Secondary Goal**: Enable manual fan control for power users who want custom cooling profiles
3. **Compatibility Goal**: Match core Macs Fan Control behaviors where technically feasible on modern macOS hardware
4. **Performance Goal**: Maintain <0.1% CPU usage when running in background (menu bar only)

### 1.3 Target Users

| User Type | Description | Primary Use Case |
|-----------|-------------|------------------|
| **General Mac Users** | Everyday users concerned about overheating | Monitor system temperature during heavy tasks |
| **Power Users** | Developers, designers, video editors | Track thermal performance during intensive workloads |
| **Gamers** | Users running graphically intensive games | Monitor GPU temps and control fan noise |
| **System Administrators** | IT professionals managing multiple Macs | Remote monitoring and thermal management |

---

## 2. User Stories

### Phase A: Read-Only Monitoring (MVP)

#### US-A1: Menu Bar Fan Speed Display
**As a** Mac user, **I want** to see current fan RPM in the menu bar, **so that** I can monitor cooling at a glance without opening an app.

- **Priority**: High
- **Acceptance Criteria**:
  - Fan RPM is displayed in menu bar icon (e.g., "2450 RPM")
  - Configurable display options: show fan RPM OR sensor temperature in menu bar
  - Updates every 2-3 seconds
  - Shows tooltip on hover with additional info
  - Icon adapts to macOS light/dark mode

#### US-A2: Temperature Dashboard
**As a** a power user, **I want** to view CPU/GPU temperatures in real-time, **so that** I can track thermal performance during intensive tasks.

- **Priority**: High
- **Acceptance Criteria**:
  - Main window shows CPU temperature (individual cores + package)
  - GPU temperature displayed if available
  - RAM temperature displayed
  - Battery temperature
  - SSD/HDD temperature (including 3rd party drives via S.M.A.R.T.)
  - Real-time graphs showing temperature history (last 60 seconds)
  - Color coding: green (normal), yellow (warm), red (hot)

#### US-A3: Low Resource Usage
**As a** user, **I want** the app to use minimal CPU when idle, **so that** it doesn't impact system performance.

- **Priority**: High
- **Acceptance Criteria**:
  - <0.1% CPU usage when only menu bar is active (window closed)
  - <3% CPU usage when main window is open
  - Efficient background polling (not busy-waiting)
  - Memory footprint <50MB

#### US-A4: Multi-Fan Display
**As a** MacBook Pro user, **I want** to see all fans (Left and Right), **so that** I understand the complete cooling state.

- **Priority**: Medium
- **Acceptance Criteria**:
  - Display RPM for each fan individually
  - Show fan names (e.g., "Left Fan", "Right Fan")
  - Indicate if a fan is not available/supported

#### US-A5: Temperature Alerts
**As a** user, **I want** to receive notifications when temperatures exceed safe thresholds, **so that** I can take action before thermal throttling occurs.

- **Priority**: Medium
- **Acceptance Criteria**:
  - Configurable temperature thresholds per sensor
  - Native macOS notification when threshold exceeded
  - Snooze/ignore option for alerts
  - Visual indicator in menu bar when temp is high

#### US-A6: Historical Data
**As a** power user, **I want** to see temperature/fan history over time, **so that** I can identify thermal patterns.

- **Priority**: Low
- **Acceptance Criteria**:
  - Store last 24 hours of data locally
  - View graphs for different time ranges (1h, 6h, 24h)
  - Export data to CSV

---

### Phase B: Fan Control (Post-MVP)

#### US-B1: Manual Fan Speed Control
**As a** user, **I want** to set a custom fan speed manually, **so that** I can balance noise vs cooling needs.

- **Priority**: High
- **Acceptance Criteria**:
  - Three fan control modes:
    - **Auto** (Default): Fan controlled by system automatically
    - **Custom (Fixed)**: Set specific RPM value
    - **Custom (Sensor-based)**: Fan speed linked to specific temperature sensor
  - Slider to set target RPM (min to max for each fan)
  - Dropdown to select target temperature sensor for sensor-based control
  - "Apply" button to confirm changes
  - "Reset to Auto" button to return to system control
  - Visual feedback showing current vs target RPM
  - Warning if setting could cause overheating

#### US-B2: Temperature-Based Fan Curves
**As a** a power user, **I want** to define custom fan curves based on temperature, **so that** fans automatically respond to heat with my preferred aggressiveness.

- **Priority**: Medium
- **Acceptance Criteria**:
  - Graph editor for custom curves (temperature -> fan speed)
  - Multiple curve points (at least 4: idle, low, medium, high)
  - Preset curves: Silent, Balanced, Performance, Maximum
  - Apply different curves to different fans
  - Save custom curves with names

#### US-B3: Fan Control Profiles
**As a** user, **I want** to save and switch between fan control profiles, **so that** I can quickly change cooling behavior for different scenarios.

- **Priority**: Medium
- **Acceptance Criteria**:
  - Pre-defined profiles:
    - **Automatic**: All fans controlled by system (default)
    - **Full Blast**: Maximum fan speed on all controllable fans
    - **Custom**: Use user-defined per-fan settings
  - Custom profile creation (define per-fan settings)
  - Quick profile switch from menu bar
  - Profile auto-switch based on power source (battery vs AC)
  - On app quit, always restore all fans to Auto mode

#### US-B4: Safe Control Limits
**As a** user, **I want** the app to prevent dangerous fan settings, **so that** I don't accidentally damage my Mac.

- **Priority**: High
- **Acceptance Criteria**:
  - Cannot set fan speed below safe minimum (e.g., 1200 RPM)
  - Emergency override: if temp exceeds critical threshold, system takes control
  - Confirmation dialog for extreme settings
  - Automatic return to auto mode if app crashes

---

## 3. Functional Requirements

### 3.1 Monitoring Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-001 | Read fan RPM from SMC for all available fans | High |
| FR-002 | Read CPU temperature (package and per-core) | High |
| FR-003 | Read GPU temperature if discrete GPU present | High |
| FR-004 | Read RAM temperature | Medium |
| FR-005 | Read battery temperature | Medium |
| FR-006 | Read SSD/storage temperature (including 3rd party S.M.A.R.T.) | Medium |
| FR-007 | Read HDD/SSD S.M.A.R.T. data for 3rd party drives | Medium |
| FR-008 | Display data in real-time (1-3 second refresh) | High |
| FR-009 | Show menu bar icon with current fan RPM | High |
| FR-010 | Open main window from menu bar click | High |

### 3.2 Control Requirements (Phase B)

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-011 | Set manual fan speed for each fan (constant RPM) | High |
| FR-012 | Return fans to automatic (system) control (Auto mode) | High |
| FR-013 | Sensor-based fan control: link fan speed to specific temperature sensor | High |
| FR-014 | Create and save custom fan curves | Medium |
| FR-015 | Apply different curves per fan | Low |
| FR-016 | Emergency thermal protection override | High |
| FR-017 | On app quit/crash, restore all fans to Auto mode | High |
| FR-018 | Support quick profile switching: Automatic / Full Blast / Custom | High |

### 3.3 UI/UX Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-019 | Native macOS menu bar integration | High |
| FR-020 | Support macOS light and dark mode | High |
| FR-021 | Keyboard shortcut to open/close window (Cmd+W, Cmd+Q) | Medium |
| FR-022 | Always-on-top option for main window | Low |
| FR-023 | Collapsible sections in main window | Medium |
| FR-024 | Responsive UI that doesn't freeze during updates | High |

---

## 4. Non-Functional Requirements

### 4.1 Performance

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-001 | CPU usage (menu bar only) | < 0.1% |
| NFR-002 | CPU usage (window open) | < 3% |
| NFR-003 | Memory footprint | < 50 MB |
| NFR-004 | UI response time | < 16ms (60 FPS) |
| NFR-005 | Data refresh interval | 1-3 seconds |

### 4.2 Compatibility

| ID | Requirement |
|----|-------------|
| NFR-006 | Support macOS 12.0 (Monterey) and later |
| NFR-007 | Support Apple Silicon (M1/M2/M3) Macs |
| NFR-008 | Support Intel Macs (if SMC interface compatible) |
| NFR-009 | Graceful degradation if sensors unavailable |
| NFR-010 | Show clear UI notice for known limited manual fan control models |

### 4.3 Security & Safety

| ID | Requirement |
|----|-------------|
| NFR-011 | Read-only operations don't require elevated permissions (Phase A) |
| NFR-012 | Fan control requires user consent and elevated permissions (Phase B) |
| NFR-013 | Thermal safety: never allow settings that could cause damage |
| NFR-014 | Handle SMC communication errors gracefully |

### 4.4 Reliability

| ID | Requirement |
|----|-------------|
| NFR-015 | App continues running if individual sensor reads fail |
| NFR-016 | Automatic recovery from SMC connection errors |
| NFR-017 | Data persistence: save user preferences |
| NFR-018 | No memory leaks over extended runtime (24+ hours) |

---

## 5. Success Criteria

### 5.1 Phase A Success Metrics

| Metric | Target |
|--------|--------|
| Temperature readings match Macs Fan Control within 2°C | 100% of readings |
| Fan RPM readings match actual SMC values | 100% of readings |
| CPU usage (idle) verified via Activity Monitor | < 0.1% |
| App launches successfully on test Macs | 100% of supported models |
| Menu bar updates visible to user | Every 2-3 seconds |

### 5.2 Phase B Success Metrics

| Metric | Target |
|--------|--------|
| Manual fan speed changes reflected in actual fan RPM | Within 5 seconds |
| Temperature-based curves respond appropriately | Tested across temp ranges |
| No thermal incidents during fan control | 0 safety issues |
| Profile switching works without app restart | 100% of switches |
| Auto-restore on quit/crash | 100% |

---

## 6. Out of Scope

The following features are intentionally excluded from initial versions:

1. **Windows/Linux support** - macOS only (SMC is Apple-specific)
2. **Remote monitoring** - Local machine only
3. **iOS companion app** - Desktop macOS only
4. **Automatic fan control algorithms** - User-defined curves only (Phase B)
5. **Machine learning predictions** - Not planned
6. **Fan diagnostics/repair** - Monitoring and control only
7. **Third-party integrations** (HomeKit, Shortcuts) - Future consideration

---

## 7. Reference

- Similar products: Macs Fan Control (crystalidea.com), Stats, iStat Menus
- Research references:
  - mac-stats (Rust + Tauri): https://github.com/raro42/mac-stats
  - macsmc crate: https://docs.rs/macsmc
  - Tauri v2 system tray: https://v2.tauri.app/learn/system-trac
  - SMC access library: https://github.com/hidden-spectrum/libsmc
  - Crystalidea Mac Fan Control features: https://crystalidea.com/macs-fan-control
  - Crystalidea fan presets: https://crystalidea.com/macs-fan-control/fan-presets
  - Crystalidea limited fan control note: https://crystalidea.com/macs-fan-control/limited-fan-control-on-some-models

---

*Document Version: 1.2*
*Last Updated: 2026-03-04*
