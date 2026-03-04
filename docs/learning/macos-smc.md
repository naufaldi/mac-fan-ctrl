# macOS System Management Controller (SMC)

Understanding the SMC, how we communicate with it, and safety considerations for hardware access.

## What is the SMC?

The System Management Controller is a chip on macOS devices that manages:
- **Thermal sensors** (CPU, GPU, battery, ambient)
- **Fan speed** (reading RPM, setting target RPM)
- **Power management** (sleep, wake, battery)
- **LED indicators** (battery, sleep)

```
┌─────────────────────────────────────┐
│           macOS System               │
│  ┌─────────────────────────────┐    │
│  │     Your Application        │    │
│  │    (mac-fan-ctrl)          │    │
│  └─────────────┬───────────────┘    │
│                │ IOKit API           │
│  ┌─────────────▼───────────────┐    │
│  │       macOS Kernel          │    │
│  │    (IOService, IOKit)       │    │
│  └─────────────┬───────────────┘    │
│                │ SMC I/O Port        │
│  ┌─────────────▼───────────────┐    │
│  │           SMC Chip          │    │
│  │    ┌─────┐ ┌─────┐ ┌─────┐  │    │
│  │    │Temp │ │ RPM │ │Fan  │  │    │
│  │    │Sens │ │Read │ │Ctrl │  │    │
│  │    └─────┘ └─────┘ └─────┘  │    │
│  └─────────────────────────────┘    │
└─────────────────────────────────────┘
```

## SMC Keys

The SMC uses 4-character keys to identify sensors and controls:

### Temperature Keys

| Key | Description | Location |
|-----|-------------|----------|
| `TC0F` | CPU Die temperature | On-die sensor |
| `TC0P` | CPU Proximity | Near CPU |
| `TG0F` | GPU Die temperature | On-die sensor |
| `TG0P` | GPU Proximity | Near GPU |
| `TB0T` | Battery temperature | Battery pack |
| `TW0P` | Airflow / Ambient | Air intake |

### Fan Keys

| Key | Description |
|-----|-------------|
| `F%dAc` | Fan %d actual RPM (0, 1, 2...) |
| `F%dTg` | Fan %d target RPM |
| `F%dMn` | Fan %d minimum RPM |
| `F%dMx` | Fan %d maximum RPM |
| `F%dSf` | Fan %d safe RPM (hardware limit) |

## Reading from SMC (Rust)

```rust
// src-tauri/src/smc.rs
use std::ffi::CString;
use mach2::kern_return::KERN_SUCCESS;
use io_kit_sys::*;

pub struct SmcInterface {
    connection: io_connect_t,
}

impl SmcInterface {
    pub fn new() -> Result<Self, SmcError> {
        // Find SMC service in IOKit registry
        let smc_port = Self::find_smc_service()?;
        let mut connection: io_connect_t = 0;
        
        // Open connection to SMC
        let result = unsafe {
            IOServiceOpen(
                smc_port,
                mach_task_self_,
                0,
                &mut connection
            )
        };
        
        if result != KERN_SUCCESS {
            return Err(SmcError::ConnectionFailed);
        }
        
        Ok(Self { connection })
    }
    
    /// Read a temperature sensor by key
    pub fn read_temperature(&self, key: &str) -> Result<f64, SmcError> {
        let data = self.read_key(key)?;
        
        // SMC returns temperature in special floating-point format
        // Convert to Celsius
        let temp = self.parse_smc_temp(data);
        Ok(temp)
    }
    
    /// Read fan RPM by fan ID
    pub fn read_fan_rpm(&self, fan_id: u8) -> Result<u16, SmcError> {
        let key = format!("F{}Ac", fan_id);
        let data = self.read_key(&key)?;
        
        // RPM is stored as 2-byte integer
        let rpm = u16::from_be_bytes([data[0], data[1]]);
        Ok(rpm)
    }
    
    fn read_key(&self, key: &str) -> Result<[u8; 32], SmcError> {
        // SMC data structure
        #[repr(C)]
        struct SmcReadStruct {
            key: [u8; 4],
            vers: u8,
            key_info: u8,
            result: u8,
            status: u8,
            data8: u8,
            data32: u32,
            bytes: [u8; 32],
        }
        
        let mut input = SmcReadStruct {
            key: Self::key_to_bytes(key)?,
            vers: 0,
            key_info: 0,
            result: 0,
            status: 0,
            data8: 0,
            data32: 0,
            bytes: [0; 32],
        };
        
        let mut output = input.clone();
        
        // Call SMC through IOKit
        let result = unsafe {
            IOConnectCallStructMethod(
                self.connection,
                KERNEL_INDEX_SMC,
                &input as *const _ as *const c_void,
                std::mem::size_of::<SmcReadStruct>(),
                &mut output as *mut _ as *mut c_void,
                &mut std::mem::size_of::<SmcReadStruct>(),
            )
        };
        
        if result != KERN_SUCCESS {
            return Err(SmcError::ReadFailed(key.to_string()));
        }
        
        Ok(output.bytes)
    }
    
    fn key_to_bytes(key: &str) -> Result<[u8; 4], SmcError> {
        if key.len() != 4 {
            return Err(SmcError::InvalidKey);
        }
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(key.as_bytes());
        Ok(bytes)
    }
}

impl Drop for SmcInterface {
    fn drop(&mut self) {
        // Always close connection when done
        unsafe {
            IOServiceClose(self.connection);
        }
    }
}
```

## Writing to SMC (Fan Control)

```rust
// src-tauri/src/smc.rs
impl SmcInterface {
    /// Set fan to automatic mode (system controlled)
    pub fn set_fan_auto(&self, fan_id: u8) -> Result<(), SmcError> {
        let key = format!("F{}Md", fan_id);
        
        // Mode 0 = Auto, 1 = Manual
        self.write_key(&key, &[0, 0, 0, 0])
    }
    
    /// Set fan target RPM (manual mode)
    pub fn set_fan_target(&self, fan_id: u8, rpm: u16) -> Result<(), SmcError> {
        // Safety: Check RPM bounds
        let max_rpm = self.get_fan_max_rpm(fan_id)?;
        if rpm > max_rpm {
            return Err(SmcError::RpmTooHigh { 
                requested: rpm, 
                max: max_rpm 
            });
        }
        
        // First set manual mode
        self.set_fan_manual(fan_id)?;
        
        // Then set target RPM
        let key = format!("F{}Tg", fan_id);
        let bytes = rpm.to_be_bytes();
        self.write_key(&key, &[bytes[0], bytes[1], 0, 0])
    }
    
    fn set_fan_manual(&self, fan_id: u8) -> Result<(), SmcError> {
        let key = format!("F{}Md", fan_id);
        self.write_key(&key, &[1, 0, 0, 0])
    }
    
    fn write_key(&self, key: &str, data: &[u8]) -> Result<(), SmcError> {
        // Similar to read but with write-specific SMC call
        // ... implementation
        
        // IMPORTANT: Always verify write succeeded by reading back
        let verified = self.read_key(key)?;
        if &verified[0..data.len()] != data {
            return Err(SmcError::WriteVerificationFailed);
        }
        
        Ok(())
    }
}
```

## Safety Layers

### 1. Hardware Limits

```rust
pub struct FanSafetyLimits {
    pub min_rpm: u16,      // Below this, fan may stall
    pub max_rpm: u16,      // Above this, hardware damage risk
    pub critical_temp: f64, // Emergency shutdown temperature
}

impl FanSafetyLimits {
    pub fn for_model(model: &str) -> Self {
        match model {
            "MacBookPro18,1" => Self {
                min_rpm: 1200,
                max_rpm: 5500,
                critical_temp: 100.0,
            },
            "MacBookPro18,2" => Self {
                min_rpm: 1200,
                max_rpm: 5500,
                critical_temp: 100.0,
            },
            // Add more models...
            _ => Self::default(),
        }
    }
}
```

### 2. Thermal Protection

```rust
pub struct ThermalGuard {
    smc: SmcInterface,
    limits: FanSafetyLimits,
}

impl ThermalGuard {
    /// Check if requested fan speed is safe given current temps
    pub fn validate_request(
        &self,
        fan_id: u8,
        target_rpm: u16
    ) -> Result<(), SafetyError> {
        let cpu_temp = self.smc.read_temperature("TC0F")?;
        let gpu_temp = self.smc.read_temperature("TG0F")?;
        let max_temp = cpu_temp.max(gpu_temp);
        
        // Prevent setting fan too low when hot
        if max_temp > 80.0 && target_rpm < 3000 {
            return Err(SafetyError::InsufficientCooling {
                temp: max_temp,
                requested_rpm: target_rpm,
            });
        }
        
        // Emergency: auto-restore if critical
        if max_temp > self.limits.critical_temp {
            self.emergency_restore(fan_id)?;
            return Err(SafetyError::CriticalTemperature(max_temp));
        }
        
        Ok(())
    }
    
    fn emergency_restore(&self, fan_id: u8) -> Result<(), SmcError> {
        log::error!("Critical temperature! Restoring auto mode for fan {}", fan_id);
        self.smc.set_fan_auto(fan_id)
    }
}
```

### 3. Cleanup on Exit

```rust
// src-tauri/src/main.rs
fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // Setup code...
            Ok(())
        })
        .on_window_event(|app, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                // Restore all fans to auto before exit
                if let Ok(smc) = SmcInterface::new() {
                    for i in 0..smc.fan_count() {
                        let _ = smc.set_fan_auto(i);
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum SmcError {
    #[error("SMC service not found")]
    ServiceNotFound,
    
    #[error("Failed to open SMC connection")]
    ConnectionFailed,
    
    #[error("Invalid SMC key: {0}")]
    InvalidKey(String),
    
    #[error("Read failed for key {0}")]
    ReadFailed(String),
    
    #[error("Write failed for key {0}")]
    WriteFailed(String),
    
    #[error("RPM {requested} exceeds maximum {max}")]
    RpmTooHigh { requested: u16, max: u16 },
    
    #[error("Write verification failed")]
    WriteVerificationFailed,
    
    #[error("Permission denied - requires elevated privileges")]
    PermissionDenied,
}
```

## Platform Differences

### Intel vs Apple Silicon

| Feature | Intel Macs | Apple Silicon |
|---------|------------|---------------|
| SMC Access | IOKit (kernel) | IOKit (kernel) |
| Key Names | Similar | Similar |
| Permission | Root sometimes | Always root for writes |
| Fan Count | 1-3 | 1-2 typically |

### Detection

```rust
pub fn detect_platform() -> Platform {
    use std::process::Command;
    
    let output = Command::new("sysctl")
        .args(&["-n", "hw.machine"])
        .output()
        .expect("Failed to detect platform");
    
    let machine = String::from_utf8_lossy(&output.stdout);
    
    if machine.starts_with("arm64") || machine.starts_with("Apple") {
        Platform::AppleSilicon
    } else {
        Platform::Intel
    }
}
```

## Testing Without Hardware

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    // Mock SMC for testing
    struct MockSmc {
        temps: HashMap<String, f64>,
        fan_rpms: HashMap<u8, u16>,
    }
    
    impl SmcOperations for MockSmc {
        fn read_temperature(&self, key: &str) -> Result<f64, SmcError> {
            self.temps.get(key).copied()
                .ok_or(SmcError::ReadFailed(key.to_string()))
        }
        
        fn read_fan_rpm(&self, fan_id: u8) -> Result<u16, SmcError> {
            self.fan_rpms.get(&fan_id).copied()
                .ok_or(SmcError::ReadFailed(format!("F{}", fan_id)))
        }
    }
    
    #[test]
    fn test_thermal_guard_blocks_low_rpm_when_hot() {
        let mock = MockSmc {
            temps: [("TC0F".to_string(), 85.0)].into(),
            fan_rpms: [(0, 4000)].into(),
        };
        
        let guard = ThermalGuard::with_interface(Box::new(mock));
        
        // Should fail: trying to set low RPM when CPU is 85°C
        let result = guard.validate_request(0, 1500);
        assert!(result.is_err());
    }
}
```

## References

- [Apple IOKit Documentation](https://developer.apple.com/documentation/iokit)
- [macOS System Management](https://developer.apple.com/library/archive/technotes/tn2169/_index.html)
- [SMC Key Reference](https://github.com/hholtmann/smcFanControl/blob/master/smc.c)
- [IOKit Fundamentals](https://developer.apple.com/library/archive/documentation/DeviceDrivers/Conceptual/IOKitFundamentals/)

## Next Steps

- [Rust Basics](./rust-basics.md) - Rust language fundamentals
- [System Programming](./system-programming.md) - macOS programming concepts
- [Tauri Architecture](./tauri-architecture.md) - Full stack overview
