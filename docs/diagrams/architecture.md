# mac-fan-ctrl System Architecture Diagrams

## 1. Component Architecture

```mermaid
flowchart TB
    subgraph Frontend["Frontend (UI Layer)"]
        MenuBar["MenuBar.svelte<br/>System Tray Icon"]
        MainWindow["MainWindow.svelte<br/>Temperature & Fan Display"]
        Settings["SettingsPanel.svelte<br/>Configuration"]
        SensorStore["sensorStore.ts<br/>Reactive Data Store"]
    end

    subgraph TauriBridge["Tauri Bridge (IPC Layer)"]
        Commands["Commands<br/>invoke()"]
        Events["Events<br/>emit/listen()"]
    end

    subgraph Backend["Rust Backend (src-tauri/)"]
        subgraph CommandHandlers["Command Handlers"]
            GetFans["get_fan_speeds()"]
            GetTemps["get_temperatures()"]
            SetFan["set_fan_speed()"]
        end

        subgraph MonitorService["Monitor Service"]
            PollTimer["Polling Timer<br/>(1-3s interval)"]
            Broadcast["Broadcast Updates"]
        end

        subgraph SmcModule["SMC Module"]
            SmcClient["SmcClient<br/>(macsmc crate wrapper)"]
            FanReader["Fan Reader"]
            TempReader["Temperature Reader"]
            FanController["Fan Controller<br/>(Phase B)"]
        end
    end

    subgraph System["macOS System"]
        SmcChip["SMC Chip<br/>(System Management Controller)"]
        Fans["Physical Fans"]
        Sensors["Temperature Sensors<br/>(CPU, GPU, Battery, SSD)"]
    end

    %% Frontend connections
    MenuBar --> SensorStore
    MainWindow --> SensorStore
    Settings --> SensorStore
    SensorStore --> Commands
    SensorStore --> Events

    %% Tauri to Backend
    Commands --> GetFans
    Commands --> GetTemps
    Commands --> SetFan
    Events --> Broadcast

    %% Backend internal
    GetFans --> SmcClient
    GetTemps --> SmcClient
    SetFan --> FanController

    PollTimer --> SmcClient
    Broadcast --> Events

    SmcClient --> FanReader
    SmcClient --> TempReader
    SmcClient --> FanController

    %% System connection
    FanReader --> SmcChip
    TempReader --> SmcChip
    FanController --> SmcChip
    SmcChip --> Fans
    SmcChip --> Sensors
```

---

## 2. Data Flow - Real-Time Monitoring

```mermaid
sequenceDiagram
    participant UI as Frontend UI
    participant Store as Sensor Store
    participant Mon as Monitor Service
    participant SMC as SMC Client
    participant Chip as macOS SMC

    Note over Mon,Chip: Background Monitoring Loop

    loop Every 2-3 seconds (menu bar mode)
        Mon->>SMC: read_fan_speeds()
        SMC->>Chip: Query SMC keys
        Chip-->>SMC: Fan RPM data
        SMC-->>Mon: Vec<FanData>

        Mon->>SMC: read_temperatures()
        SMC->>Chip: Query SMC keys
        Chip-->>SMC: Temperature data
        SMC-->>Mon: TemperatureData

        Mon->>Store: Broadcast update (Tauri Event)
        Store->>UI: Reactive update
    end

    Note over UI,Store: On-Demand Requests

    UI->>Store: Request current data
    Store->>Mon: get_current_data()
    Mon-->>Store: Latest sensor data
    Store-->>UI: Display update
```

---

## 3. Data Flow - Manual Fan Control (Phase B)

```mermaid
sequenceDiagram
    participant User as User
    participant UI as Settings UI
    participant Cmd as Tauri Command
    participant Val as Validation Layer
    participant SMC as SMC Client
    participant Chip as macOS SMC
    participant Safe as Safety Monitor

    User->>UI: Adjust fan speed slider
    UI->>Cmd: set_fan_speed(fan_id, rpm)

    Cmd->>Val: Validate request
    alt Invalid RPM
        Val-->>Cmd: Error: InvalidRpm
        Cmd-->>UI: Show error message
    else Valid RPM
        Val->>SMC: Set target speed
        SMC->>Chip: Write SMC key
        Chip-->>SMC: Confirm
        SMC-->>Val: Success
        Val-->>Cmd: OK
        Cmd-->>UI: Update display

        Note over Safe: Thermal Safety Check
        Safe->>SMC: Read CPU temperature
        SMC->>Chip: Query temp
        Chip-->>SMC: Temperature
        alt Temp > 95C
            Safe->>SMC: Emergency: Set auto mode
            SMC->>Chip: Disable manual control
            SMC-->>Safe: Confirmed
            Safe-->>UI: Alert: Safety override
        end
    end
```

---

## 4. Module Dependencies

```mermaid
flowchart LR
    subgraph FrontendModules["Frontend (TypeScript/Svelte)"]
        Svelte["Svelte Components"]
        Stores["Svelte Stores"]
        TauriAPI["@tauri-apps/api"]
        ChartLib["Chart.js (graphs)"]
    end

    subgraph TauriRuntime["Tauri Runtime"]
        Webview["Webview (Wry)"]
        IPC["IPC Router"]
        Tray["System Tray"]
    end

    subgraph RustModules["Rust Modules"]
        Commands["commands.rs"]
        Monitor["monitor.rs"]
        Smc["smc.rs"]
        Main["main.rs"]
    end

    subgraph ExternalCrates["External Crates"]
        Macsmc["macsmc"]
        Tokio["tokio (async)"]
        Serde["serde (serialization)"]
        TauriLib["tauri"]
    end

    Svelte --> Stores
    Stores --> TauriAPI
    Svelte --> ChartLib

    TauriAPI --> IPC
    IPC --> Commands
    TauriRuntime --> Tray

    Commands --> Smc
    Commands --> Monitor
    Monitor --> Smc
    Main --> Commands
    Main --> Monitor
    Main --> Tray

    Smc --> Macsmc
    Monitor --> Tokio
    Commands --> Serde
    Main --> TauriLib
```

---

## 5. State Management

```mermaid
flowchart TB
    subgraph GlobalState["Global Application State"]
        AppState["AppState<br/>(Tauri Managed State)"]

        subgraph StateComponents["State Components"]
            SmcState["SmcClient<br/>(Mutex<Smc>)"]
            ConfigState["Config<br/>(User Settings)"]
            MonitorState["MonitorService<br/>(Background Task)"]
        end
    end

    subgraph FrontendState["Frontend State (Svelte)"]
        SensorState["sensorStore<br/>(Current readings)"]
        UIState["uiStore<br/>(Window state, theme)"]
        ConfigStore["settingsStore<br/>(User preferences)"]
    end

    AppState --> SmcState
    AppState --> ConfigState
    AppState --> MonitorState

    MonitorState -.->|Broadcast| SensorState
    SmcState -.->|On-demand| SensorState
    ConfigState -.->|Sync| ConfigStore
    UIState -->|Read/Write| ConfigStore
```

---

## 6. Menu Bar / System Tray Architecture

```mermaid
flowchart TB
    subgraph TrayMenu["System Tray Menu"]
        Icon["Tray Icon<br/>(Template for dark/light mode)"]

        subgraph MenuItems["Menu Items"]
            ShowWindow["Show Main Window"]
            CurrentTemps["Current Temps<br/>(Read-only display)"]
            CurrentFans["Current Fans<br/>(Read-only display)"]
            Divider["---"]
            ProfileSelector["Profile: Auto / Manual"]
            Divider2["---"]
            Preferences["Preferences..."]
            Quit["Quit"]
        end
    end

    subgraph ClickHandlers["Click Handlers"]
        LeftClick["Left Click<br/>Toggle Window"]
        RightClick["Right Click<br/>Show Menu"]
    end

    subgraph Backend["Backend Actions"]
        ToggleWindow["toggle_window()"]
        GetQuickData["get_quick_stats()<br/>(Lightweight poll)"]
    end

    Icon --> MenuItems
    Icon --> LeftClick
    Icon --> RightClick

    ShowWindow --> ToggleWindow
    CurrentTemps --> GetQuickData
    CurrentFans --> GetQuickData
```

---

## 7. Deployment Architecture

```mermaid
flowchart LR
    subgraph Dev["Development"]
        DevCode["Source Code"]
        DevBuild["cargo tauri dev<br/>(Hot reload)"]
    end

    subgraph Build["Build Process"]
        RustBuild["cargo build --release"]
        FrontendBuild["Vite build"]
        Bundle["tauri bundle"]
    end

    subgraph Output["Output"]
        DMG["mac-fan-ctrl.dmg<br/>(Disk Image)"]
        App["mac-fan-ctrl.app<br/>(Application Bundle)"]
        Binary["mac-fan-ctrl<br/>(Unix Executable)"]
    end

    subgraph Distribution["Distribution"]
        GitHub["GitHub Releases"]
        Homebrew["Homebrew Cask<br/>(Future)"]
    end

    DevCode --> DevBuild
    DevBuild -->|Release build| RustBuild
    DevCode --> FrontendBuild
    RustBuild --> Bundle
    FrontendBuild --> Bundle
    Bundle --> DMG
    Bundle --> App
    App --> Binary
    DMG --> GitHub
    DMG --> Homebrew
```

---

## 8. Error Handling Flow

```mermaid
flowchart TB
    subgraph ErrorSources["Error Sources"]
        SmcErr["SMC Communication"]
        PermissionErr["Permission Denied"]
        SafetyErr["Thermal Safety"]
        ValidationErr["Input Validation"]
    end

    subgraph ErrorHandling["Error Handling"]
        RustErr["Rust Layer<br/>(thiserror)"]
        TauriErr["Tauri Command Result"]
        FrontendErr["Frontend Handler"]
    end

    subgraph UserExperience["User Experience"]
        Toast["Toast Notification"]
        Modal["Error Modal"]
        Log["Console Log"]
        Fallback["Graceful Fallback"]
    end

    SmcErr --> RustErr
    PermissionErr --> RustErr
    SafetyErr --> RustErr
    ValidationErr --> RustErr

    RustErr --> TauriErr
    TauriErr --> FrontendErr

    FrontendErr --> Toast
    FrontendErr --> Modal
    FrontendErr --> Log
    FrontendErr --> Fallback
```

---

*Diagrams created with Mermaid syntax*
*Version: 1.0*
