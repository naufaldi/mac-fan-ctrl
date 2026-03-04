# Rust Basics for mac-fan-ctrl

This guide covers the Rust fundamentals you need to understand and contribute to mac-fan-ctrl.

## Why Rust?

```rust
// Rust gives us memory safety without garbage collection
// Perfect for system-level hardware interaction
fn read_temperature() -> Result<f64, SmcError> {
    // Safe, performant, zero-cost abstractions
    let smc = Smc::open()?;
    smc.read_key("TC0F")  // CPU temperature
}
```

## Core Concepts

### 1. Variables and Mutability

```rust
// Immutable by default (safer, clearer)
let temperature = 45.5;
// temperature = 46.0; // ERROR: cannot assign twice

// Explicit mutability when needed
let mut fan_speed = 2000;
fan_speed = 2500; // OK

// Type inference works, but you can be explicit
let rpm: u16 = 3500;
```

### 2. Ownership (The Big Idea)

```rust
// Each value has ONE owner
let data = vec![1, 2, 3];  // data owns the vector

// Moving ownership
let new_owner = data;      // data is MOVED, no longer valid
// println!("{:?}", data); // ERROR: data was moved

// Borrowing instead of moving
let data2 = vec![4, 5, 6];
print_length(&data2);      // Borrow with &
print_length(&data2);       // Can borrow multiple times (immutable)
// data2 still valid here

fn print_length(v: &Vec<i32>) {
    println!("Length: {}", v.len());
}
```

**Real example from mac-fan-ctrl:**

```rust
// src-tauri/src/commands.rs
pub fn get_fan_speeds(state: &AppState) -> Vec<FanInfo> {
    // state is borrowed, not moved
    // We can read from it without taking ownership
    state.monitor_service.get_fans()
}
```

### 3. Structs and Methods

```rust
// Define data structures
pub struct FanInfo {
    pub id: u8,
    pub name: String,
    pub current_rpm: u16,
    pub target_rpm: u16,
    pub mode: FanMode,
}

// Implementation (methods)
impl FanInfo {
    // Constructor-like associated function
    pub fn new(id: u8, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            current_rpm: 0,
            target_rpm: 0,
            mode: FanMode::Auto,
        }
    }

    // Method with &self (borrows immutably)
    pub fn is_spinning(&self) -> bool {
        self.current_rpm > 0
    }

    // Method with &mut self (borrows mutably)
    pub fn set_target(&mut self, rpm: u16) {
        self.target_rpm = rpm;
    }
}
```

### 4. Enums and Pattern Matching

```rust
// Enums with data (Algebraic Data Types)
pub enum FanMode {
    Auto,
    Manual { target_rpm: u16 },
    Curve { sensor: String, curve: TemperatureCurve },
}

// Pattern matching with match
fn describe_mode(mode: &FanMode) -> String {
    match mode {
        FanMode::Auto => "Automatic control".to_string(),
        FanMode::Manual { target_rpm } => {
            format!("Manual at {} RPM", target_rpm)
        }
        FanMode::Curve { sensor, .. } => {
            format!("Curve based on {}", sensor)
        }
    }
}

// Exhaustive matching (compiler checks all cases)
fn get_target_rpm(mode: &FanMode) -> Option<u16> {
    match mode {
        FanMode::Auto => None,
        FanMode::Manual { target_rpm } => Some(*target_rpm),
        FanMode::Curve { .. } => None, // Curve calculates dynamically
    }
}
```

### 5. Error Handling

```rust
// Result type: Ok(T) or Err(E)
fn read_smc_key(key: &str) -> Result<u32, SmcError> {
    let smc = Smc::open()?;  // ? propagates errors
    smc.read(key)
}

// Option type: Some(T) or None
fn find_fan_by_id(fans: &[FanInfo], id: u8) -> Option<&FanInfo> {
    fans.iter().find(|f| f.id == id)
}

// Combining Result and Option
fn get_fan_speed(fans: &[FanInfo], id: u8) -> Result<u16, String> {
    let fan = find_fan_by_id(fans, id)
        .ok_or("Fan not found")?;
    
    Ok(fan.current_rpm)
}
```

### 6. Traits (Interfaces)

```rust
// Define shared behavior
pub trait Sensor {
    fn read(&self) -> Result<f64, SensorError>;
    fn name(&self) -> &str;
}

// Implement for different types
pub struct CpuSensor { key: String }
impl Sensor for CpuSensor {
    fn read(&self) -> Result<f64, SensorError> {
        // Read from SMC key
        read_temperature(&self.key)
    }
    
    fn name(&self) -> &str {
        &self.key
    }
}

// Use traits as bounds
fn read_all_sensors(sensors: &[&dyn Sensor]) -> Vec<f64> {
    sensors.iter()
        .filter_map(|s| s.read().ok())
        .collect()
}
```

### 7. Collections

```rust
// Vector (dynamic array)
let mut fans: Vec<FanInfo> = Vec::new();
fans.push(FanInfo::new(0, "Left Fan"));
fans.push(FanInfo::new(1, "Right Fan"));

// HashMap (key-value store)
use std::collections::HashMap;

let mut profiles: HashMap<String, FanProfile> = HashMap::new();
profiles.insert("quiet".to_string(), FanProfile::quiet());
profiles.insert("performance".to_string(), FanProfile::performance());

// Iteration
for fan in &fans {
    println!("{}: {} RPM", fan.name, fan.current_rpm);
}

// Functional style
let total_rpm: u32 = fans.iter()
    .map(|f| f.current_rpm as u32)
    .sum();
```

### 8. Lifetimes

```rust
// Lifetimes ensure references don't outlive data they point to
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() { x } else { y }
}

// In struct definitions
pub struct MonitorService<'a> {
    smc: &'a Smc,
    poll_interval: Duration,
}

// The compiler often infers lifetimes (elision)
fn get_name(fan: &FanInfo) -> &str {  // same as &'a FanInfo -> &'a str
    &fan.name
}
```

## Common Patterns in mac-fan-ctrl

### State Management

```rust
// Thread-safe shared state with Arc + Mutex
use std::sync::{Arc, Mutex};

pub struct AppState {
    monitor: Arc<Mutex<MonitorService>>,
}

// Cloning Arc just clones the pointer, not the data
let state_clone = Arc::clone(&state.monitor);
```

### Configuration

```rust
// Builder pattern
pub struct FanCurveBuilder {
    points: Vec<(f64, u16)>,
}

impl FanCurveBuilder {
    pub fn new() -> Self {
        Self { points: Vec::new() }
    }
    
    pub fn add_point(mut self, temp: f64, rpm: u16) -> Self {
        self.points.push((temp, rpm));
        self
    }
    
    pub fn build(self) -> FanCurve {
        FanCurve { points: self.points }
    }
}

// Usage
let curve = FanCurveBuilder::new()
    .add_point(50.0, 2000)
    .add_point(70.0, 4000)
    .add_point(90.0, 6000)
    .build();
```

## Next Steps

- [Rust Ownership & Borrowing](./rust-ownership.md) - Deep dive into ownership
- [Error Handling in Rust](./rust-errors.md) - Result/Option patterns
- [Async Rust](./async-rust.md) - Tokio and async/await
- [macOS SMC](./macos-smc.md) - Hardware interaction
