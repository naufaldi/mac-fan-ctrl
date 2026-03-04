# System Programming for macOS

Core concepts for low-level macOS development: IOKit, memory management, and safe hardware access.

## What is System Programming?

System programming involves writing software that:
- Interfaces directly with hardware
- Manages memory and resources
- Operates at the kernel/user boundary
- Requires understanding of OS internals

```
User Space (Your App)          Kernel Space
┌─────────────────────┐       ┌─────────────────────┐
│  mac-fan-ctrl UI    │       │     XNU Kernel      │
│  (Svelte/TS)        │       │  ┌───────────────┐  │
└─────────┬───────────┘       │  │  IOKit        │  │
          │ Tauri Bridge      │  │  Scheduler    │  │
┌─────────▼───────────┐      │  │  Memory Mgmt  │  │
│  Rust Backend         │◄────►│  │  Drivers      │  │
│  (src-tauri)          │      │  └───────┬───────┘  │
└─────────┬───────────┘      │          │          │
          │ IOKit API        │  ┌───────▼───────┐  │
┌─────────▼───────────┐      │  │  SMC Driver   │  │
│  IOKit Framework    │◄────►│  │  (Hardware)   │  │
│  (Apple library)    │      │  └───────────────┘  │
└─────────────────────┘      └─────────────────────┘
```

## IOKit Framework

IOKit is Apple's framework for device drivers and hardware communication.

### Service Discovery

```rust
use io_kit_sys::*;
use mach2::kern_return::KERN_SUCCESS;

/// Find a hardware service by name
pub fn find_service_by_name(name: &str) -> Result<io_service_t, IoError> {
    // Create a matching dictionary
    let matching = unsafe {
        IOServiceMatching(CString::new(name)?.as_ptr())
    };
    
    if matching.is_null() {
        return Err(IoError::ServiceNotFound(name.to_string()));
    }
    
    // Search for the service in the I/O Registry
    let service = unsafe {
        IOServiceGetMatchingService(kIOMasterPortDefault, matching)
    };
    
    if service == 0 {
        return Err(IoError::ServiceNotFound(name.to_string()));
    }
    
    Ok(service)
}

// Usage: Find the Apple SMC
let smc = find_service_by_name("AppleSMC")?;
```

### Registry Properties

```rust
/// Read property from I/O Registry
pub fn get_property<T: FromProperty>(
    service: io_service_t,
    key: &str
) -> Result<T, IoError> {
    let key_cstring = CString::new(key)?;
    
    let property = unsafe {
        IORegistryEntryCreateCFProperty(
            service,
            key_cstring.as_ptr() as CFStringRef,
            kCFAllocatorDefault,
            0,
        )
    };
    
    if property.is_null() {
        return Err(IoError::PropertyNotFound(key.to_string()));
    }
    
    // Convert CFType to Rust type
    T::from_cfproperty(property)
}

// Usage
let model = get_property::<String>(platform, "model")?;
let board_id = get_property::<String>(platform, "board-id")?;
```

## Memory Safety in System Code

### Raw Pointers

```rust
// Unsafe: dealing with raw pointers
unsafe fn read_memory_mapped_register(addr: *mut u32) -> u32 {
    // Volatile read (may change outside our control)
    addr.read_volatile()
}

// Safe wrapper with bounds checking
pub struct MmioRegion {
    base: *mut u8,
    size: usize,
}

impl MmioRegion {
    pub fn read_u32(&self, offset: usize) -> Result<u32, IoError> {
        if offset + 4 > self.size {
            return Err(IoError::OutOfBounds);
        }
        
        let addr = unsafe { self.base.add(offset) as *mut u32 };
        Ok(unsafe { addr.read_volatile() })
    }
}

// RAII guard for memory-mapped regions
impl Drop for MmioRegion {
    fn drop(&mut self) {
        // Unmap memory when done
        unsafe {
            munmap(self.base as *mut c_void, self.size);
        }
    }
}
```

### FFI (Foreign Function Interface)

```rust
// Link to system libraries
#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IOServiceOpen(
        service: io_service_t,
        owning_task: mach_port_t,
        connect_type: u32,
        connection: *mut io_connect_t,
    ) -> kern_return_t;
    
    fn IOConnectCallStructMethod(
        connection: io_connect_t,
        selector: u32,
        input: *const c_void,
        input_size: usize,
        output: *mut c_void,
        output_size: *mut usize,
    ) -> kern_return_t;
}

// Safe wrapper
pub struct IoConnection {
    service: io_service_t,
    connect: io_connect_t,
}

impl IoConnection {
    pub fn new(service: io_service_t) -> Result<Self, IoError> {
        let mut connect: io_connect_t = 0;
        
        let result = unsafe {
            IOServiceOpen(
                service,
                mach_task_self_,
                0,
                &mut connect,
            )
        };
        
        if result != KERN_SUCCESS {
            return Err(IoError::ConnectionFailed(result));
        }
        
        Ok(Self { service, connect })
    }
    
    pub fn call_method(
        &self,
        selector: u32,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<(), IoError> {
        let mut output_size = output.len();
        
        let result = unsafe {
            IOConnectCallStructMethod(
                self.connect,
                selector,
                input.as_ptr() as *const c_void,
                input.len(),
                output.as_mut_ptr() as *mut c_void,
                &mut output_size,
            )
        };
        
        if result != KERN_SUCCESS {
            return Err(IoError::CallFailed(result));
        }
        
        Ok(())
    }
}

impl Drop for IoConnection {
    fn drop(&mut self) {
        unsafe {
            IOServiceClose(self.connect);
            IOObjectRelease(self.service);
        }
    }
}
```

## Concurrency and Async

### Tokio for Async I/O

```rust
use tokio::time::{interval, Duration};
use tokio::sync::broadcast;

pub struct MonitorService {
    smc: Arc<Mutex<SmcInterface>>,
    tx: broadcast::Sender<TelemetryData>,
}

impl MonitorService {
    pub async fn run(&self) {
        let mut ticker = interval(Duration::from_secs(1));
        
        loop {
            ticker.tick().await;
            
            // Read sensors
            let data = self.collect_telemetry().await;
            
            // Broadcast to all listeners
            let _ = self.tx.send(data);
        }
    }
    
    async fn collect_telemetry(&self) -> TelemetryData {
        // Lock SMC for reading
        let smc = self.smc.lock().await;
        
        TelemetryData {
            cpu_temp: smc.read_temperature("TC0F").ok(),
            gpu_temp: smc.read_temperature("TG0F").ok(),
            fans: smc.read_all_fans().ok().unwrap_or_default(),
            timestamp: Instant::now(),
        }
    }
}

// Spawn monitoring task
tokio::spawn(async move {
    monitor.run().await;
});
```

### Thread Safety

```rust
use std::sync::{Arc, RwLock};
use crossbeam::channel;

// Shared state between threads
pub struct SharedState {
    config: Arc<RwLock<AppConfig>>,
    command_tx: channel::Sender<Command>,
}

impl SharedState {
    /// Read config (many readers allowed)
    pub fn get_config(&self) -> AppConfig {
        self.config.read().unwrap().clone()
    }
    
    /// Update config (exclusive access)
    pub fn update_config(&self, update: ConfigUpdate) {
        let mut config = self.config.write().unwrap();
        config.apply(update);
    }
    
    /// Send command to worker thread
    pub fn send_command(&self, cmd: Command) -> Result<(), SendError<Command>> {
        self.command_tx.send(cmd)
    }
}
```

## macOS Security Model

### Sandboxing and Entitlements

```xml
<!-- src-tauri/Info.plist -->
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <!-- Required for IOKit access -->
    <key>com.apple.security.cs.allow-jit</key>
    <true/>
    
    <!-- Allow hardware access -->
    <key>com.apple.security.temporary-exception.mach-lookup.global-name</key>
    <array>
        <string>com.apple.systempreferences</string>
    </array>
</dict>
</plist>
```

```rust
// tauri.conf.json
{
  "bundle": {
    "macOS": {
      "entitlements": "./Info.plist",
      "signingIdentity": null,
      "providerShortName": null
    }
  },
  "permissions": [
    {
      "identifier": "allow-execute",
      "allow": [{"cmd": "smc"}]
    }
  ]
}
```

### Elevated Privileges

```rust
use security_framework::authorization::*;

pub fn request_elevation(reason: &str) -> Result<(), AuthError> {
    let auth = Authorization::new(
        &[
            "system.privilege.admin".to_string(),
            "com.apple.system.smc".to_string(),
        ],
        AuthorizationFlags::DEFAULTS,
    )?;
    
    auth.obtain_with_prompt(reason)?;
    
    // Now we have elevated privileges
    Ok(())
}

// Usage before SMC write
if !has_elevation() {
    request_elevation("mac-fan-ctrl needs permission to control fans")?;
}
```

## Error Handling at System Level

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SystemError {
    #[error("IOKit error: {0}")]
    IoKit(#[from] IoError),
    
    #[error("Permission denied: {0}")]
    Permission(String),
    
    #[error("Hardware not found: {0}")]
    HardwareNotFound(String),
    
    #[error("Invalid state: {0}")]
    InvalidState(String),
    
    #[error("Timeout waiting for {0}")]
    Timeout(String),
}

// Kernel return codes
#[derive(Error, Debug)]
pub enum IoError {
    #[error("Service not found: {0}")]
    ServiceNotFound(String),
    
    #[error("Connection failed (code: {0})")]
    ConnectionFailed(kern_return_t),
    
    #[error("Call failed (code: {0})")]
    CallFailed(kern_return_t),
    
    #[error("Property not found: {0}")]
    PropertyNotFound(String),
    
    #[error("Out of bounds access")]
    OutOfBounds,
    
    #[error("Invalid string")]
    InvalidString(#[from] std::ffi::NulError),
}

impl From<kern_return_t> for IoError {
    fn from(code: kern_return_t) -> Self {
        match code {
            KERN_SUCCESS => unreachable!(),
            KERN_INVALID_ARGUMENT => IoError::CallFailed(code),
            _ => IoError::CallFailed(code),
        }
    }
}
```

## Performance Considerations

### Minimizing Syscalls

```rust
pub struct BatchedReader {
    smc: SmcInterface,
    cache: HashMap<String, (Value, Instant)>,
    ttl: Duration,
}

impl BatchedReader {
    /// Read multiple keys efficiently
    pub fn read_batch(&mut self, keys: &[&str]) -> Result<Vec<Value>, SmcError> {
        let mut results = Vec::with_capacity(keys.len());
        let now = Instant::now();
        
        for key in keys {
            // Check cache first
            if let Some((value, timestamp)) = self.cache.get(*key) {
                if now.duration_since(*timestamp) < self.ttl {
                    results.push(value.clone());
                    continue;
                }
            }
            
            // Read from hardware
            let value = self.smc.read_key(key)?;
            self.cache.insert(key.to_string(), (value.clone(), now));
            results.push(value);
        }
        
        Ok(results)
    }
}
```

### Zero-Copy where possible

```rust
// Avoid allocations in hot paths
pub fn parse_smc_data(data: &[u8; 32]) -> SensorReading {
    SensorReading {
        // Borrow instead of copy when possible
        raw: data,
        // Parse on demand
        parsed: None,
    }
}
```

## Next Steps

- [macOS SMC](./macos-smc.md) - Specific SMC implementation
- [Rust Ownership](./rust-ownership.md) - Memory management deep dive
- [Async Rust](./async-rust.md) - Tokio and concurrent programming
