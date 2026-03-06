//! Raw IOKit FFI for writing SMC keys.
//!
//! The `macsmc` crate is read-only — it exposes no write methods.
//! This module opens a parallel SMC connection and implements
//! `write_key` via `IOConnectCallStructMethod`.
//!
//! Fan-relevant SMC keys:
//!   F{n}Md — fan mode  (ui8: 0 = Auto, 1 = Forced, 3 = System/AS)
//!   F{n}Tg — fan target RPM (flt / fpe2)
//!   Ftst   — force-test diagnostic flag (ui8: 1 = unlock fan control)
//!
//! On Apple Silicon (M3/M4+), `thermalmonitord` blocks direct fan
//! mode writes. The `Ftst` key must be set to 1 first to inhibit
//! thermal enforcement, then fan mode/target writes take effect.

use std::os::raw::c_void;
use thiserror::Error;

// ── Error types ──────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum SmcWriteError {
    #[error("SMC device not found — not running on a Mac?")]
    DeviceNotFound,
    #[error("Failed to open SMC connection: {0:#010x}")]
    OpenFailed(i32),
    #[error("Insufficient privileges — SMC writes require root")]
    InsufficientPrivileges,
    #[error("Unknown key: {0}")]
    UnknownKey(String),
    #[error("SMC call failed: result={0:#010x} smc_result={1}")]
    CallFailed(i32, u8),
    #[error("Invalid fan index: {0}")]
    InvalidFanId(u8),
    #[error("RPM {requested} out of bounds [{min}, {max}]")]
    InvalidRpm { min: f32, max: f32, requested: f32 },
}

// ── Low-level IOKit FFI types ────────────────────────────────────────────────
//
// macOS defines mach_port_t, io_connect_t, io_service_t, etc. as `unsigned int`
// (4 bytes), NOT pointers.

#[allow(non_camel_case_types)]
type kern_return_t = i32;
#[allow(non_camel_case_types)]
type mach_port_t = u32;
#[allow(non_camel_case_types)]
type io_connect_t = u32;
#[allow(non_camel_case_types)]
type io_service_t = u32;
#[allow(non_camel_case_types)]
type io_object_t = u32;
#[allow(non_camel_case_types)]
type task_port_t = u32;

type CFMutableDictionaryRef = *mut c_void;
type CFDictionaryRef = *const c_void;

const MACH_PORT_NULL: mach_port_t = 0;
const MASTER_PORT_DEFAULT: mach_port_t = MACH_PORT_NULL;
const KERN_SUCCESS: kern_return_t = 0;
const SYS_IOKIT: kern_return_t = (0x38 & 0x3f) << 26;
const SUB_IOKIT_COMMON: kern_return_t = 0;
const RETURN_NOT_PRIVILEGED: kern_return_t = SYS_IOKIT | SUB_IOKIT_COMMON | 0x2c1;
const KERNEL_INDEX_SMC: u32 = 2;

const SMC_CMD_READ_KEYINFO: u8 = 9;
#[allow(dead_code)]
const SMC_CMD_READ_BYTES: u8 = 5;
const SMC_CMD_WRITE_BYTES: u8 = 6;

#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IOServiceMatching(name: *const u8) -> CFMutableDictionaryRef;
    fn IOServiceGetMatchingService(
        master_port: mach_port_t,
        matching: CFDictionaryRef,
    ) -> io_service_t;
    fn IOServiceOpen(
        service: io_service_t,
        owning_task: task_port_t,
        r#type: u32,
        connect: *mut io_connect_t,
    ) -> kern_return_t;
    fn IOServiceClose(connect: io_connect_t) -> kern_return_t;
    fn IOConnectCallStructMethod(
        connection: io_connect_t,
        selector: u32,
        input: *const c_void,
        input_size: usize,
        output: *mut c_void,
        output_size: *mut usize,
    ) -> kern_return_t;
    fn IOObjectRelease(object: io_object_t) -> kern_return_t;
    fn mach_task_self() -> task_port_t;
}

// ── SMC data structures (matches Apple's kernel interface) ───────────────────

#[derive(Copy, Clone, Debug, Default)]
#[repr(C)]
struct SmcKeyDataVersion {
    major: u8,
    minor: u8,
    build: u8,
    reserved: u8,
    release: u16,
}

#[derive(Copy, Clone, Debug, Default)]
#[repr(C)]
struct SmcKeyDataLimitData {
    version: u16,
    length: u16,
    cpu_p_limit: u32,
    gpu_p_limit: u32,
    mem_p_limit: u32,
}

#[derive(Copy, Clone, Debug, Default)]
#[repr(C)]
struct SmcKeyDataKeyInfo {
    data_size: u32,
    data_type: u32,
    data_attributes: u8,
}

#[derive(Copy, Clone, Debug, Default)]
#[repr(C)]
struct SmcBytes([u8; 32]);

#[derive(Copy, Clone, Debug, Default)]
#[repr(C)]
struct SmcKeyData {
    key: u32,
    version: SmcKeyDataVersion,
    p_limit_data: SmcKeyDataLimitData,
    key_info: SmcKeyDataKeyInfo,
    result: u8,
    status: u8,
    data8: u8,
    data32: u32,
    bytes: SmcBytes,
}

// ── Public writer struct ─────────────────────────────────────────────────────

/// Owns a separate IOKit connection to `AppleSMC` for writing keys.
pub struct SmcWriter {
    conn: io_connect_t,
}

// Safety: The IOKit connection handle is a kernel port that is safe to use
// from any thread. All SMC calls are serialized through `Mutex<SmcWriter>`.
unsafe impl Send for SmcWriter {}
unsafe impl Sync for SmcWriter {}

impl Drop for SmcWriter {
    fn drop(&mut self) {
        // Best-effort: re-lock thermal control on shutdown
        let _ = self.lock_fan_control();
        unsafe {
            let _ = IOServiceClose(self.conn);
        }
    }
}

impl SmcWriter {
    /// Opens a new connection to the AppleSMC kernel service.
    pub fn new() -> Result<Self, SmcWriteError> {
        let conn = unsafe { smc_open() }?;
        Ok(Self { conn })
    }

    // ── Apple Silicon thermal unlock ─────────────────────────────────────

    /// Sets the `Ftst` (force-test) diagnostic flag to inhibit
    /// `thermalmonitord` thermal enforcement, allowing fan mode writes.
    pub fn unlock_fan_control(&self) -> Result<(), SmcWriteError> {
        let key = u32::from_be_bytes(*b"Ftst");
        let key_info = self.read_key_info(key)?;
        eprintln!("[smc_writer] unlock_fan_control: writing Ftst=1");
        self.write_key_bytes(key, key_info.data_size, &[1])
    }

    /// Clears the `Ftst` flag, re-enabling thermal enforcement.
    pub fn lock_fan_control(&self) -> Result<(), SmcWriteError> {
        let key = u32::from_be_bytes(*b"Ftst");
        let key_info = self.read_key_info(key)?;
        eprintln!("[smc_writer] lock_fan_control: writing Ftst=0");
        self.write_key_bytes(key, key_info.data_size, &[0])
    }

    // ── Fan control helpers ──────────────────────────────────────────────

    /// Sets fan to forced mode and writes a target RPM.
    ///
    /// Strategy (adapts to chip generation):
    ///   1. Try `Ftst=1` unlock (M3/M4 need it; gracefully skipped if key absent)
    ///   2. Write `F{n}Md=1` (forced mode)
    ///   3. Write `F{n}Tg=<rpm>`
    pub fn set_fan_target_rpm(
        &self,
        fan_index: u8,
        rpm: f32,
        min_rpm: f32,
        max_rpm: f32,
    ) -> Result<(), SmcWriteError> {
        eprintln!("[smc_writer] set_fan_target_rpm: fan={fan_index} rpm={rpm} bounds=[{min_rpm}, {max_rpm}]");
        if rpm < min_rpm || rpm > max_rpm {
            return Err(SmcWriteError::InvalidRpm {
                min: min_rpm,
                max: max_rpm,
                requested: rpm,
            });
        }

        // Step 1: Unlock thermal enforcement (best-effort — key may not exist on M1)
        match self.unlock_fan_control() {
            Ok(()) => eprintln!("[smc_writer] Ftst unlock OK"),
            Err(SmcWriteError::UnknownKey(_)) => {
                eprintln!("[smc_writer] Ftst key not present (M1) — skipping unlock");
            }
            Err(e) => {
                eprintln!("[smc_writer] Ftst unlock failed: {e} — continuing anyway");
            }
        }

        // Step 2: Set forced mode
        eprintln!("[smc_writer] Setting F{fan_index}Md=1 (forced)");
        self.set_fan_mode(fan_index, true)?;

        // Step 3: Write target RPM
        let key = fan_key(fan_index, b"Tg");
        let key_info = self.read_key_info(key)?;
        let type_bytes = key_info.data_type.to_be_bytes();
        let type_str = String::from_utf8_lossy(&type_bytes);
        eprintln!("[smc_writer] F{fan_index}Tg type={type_str} size={}", key_info.data_size);
        let bytes = encode_value(rpm, key_info.data_type, key_info.data_size)?;
        self.write_key_bytes(key, key_info.data_size, &bytes)?;
        eprintln!("[smc_writer] set_fan_target_rpm: done fan={fan_index} rpm={rpm}");
        Ok(())
    }

    /// Returns fan to automatic (system-controlled) mode.
    /// Caller is responsible for calling `lock_fan_control()` when
    /// no fans remain in forced mode.
    pub fn set_fan_auto(&self, fan_index: u8) -> Result<(), SmcWriteError> {
        eprintln!("[smc_writer] set_fan_auto: fan={fan_index}");
        self.set_fan_mode(fan_index, false)
    }

    /// Sets the fan mode flag: `false` = Auto, `true` = Forced.
    fn set_fan_mode(&self, fan_index: u8, forced: bool) -> Result<(), SmcWriteError> {
        let key = fan_key(fan_index, b"Md");
        let key_info = self.read_key_info(key)?;
        let value: u8 = if forced { 1 } else { 0 };
        self.write_key_bytes(key, key_info.data_size, &[value])
    }

    // ── Low-level SMC operations ─────────────────────────────────────────

    /// Reads key info (data type + size) for a given 4-char SMC key.
    fn read_key_info(&self, key: u32) -> Result<SmcKeyDataKeyInfo, SmcWriteError> {
        let mut input = SmcKeyData::default();
        input.key = key;
        input.data8 = SMC_CMD_READ_KEYINFO;

        let mut output = SmcKeyData::default();
        self.smc_call(&input, &mut output)?;

        Ok(output.key_info)
    }

    /// Reads raw bytes from an SMC key.
    #[allow(dead_code)]
    fn read_key_bytes(
        &self,
        key: u32,
        data_size: u32,
    ) -> Result<Vec<u8>, SmcWriteError> {
        let mut input = SmcKeyData::default();
        input.key = key;
        input.data8 = SMC_CMD_READ_BYTES;
        input.key_info.data_size = data_size;

        let mut output = SmcKeyData::default();
        self.smc_call(&input, &mut output)?;

        let len = data_size as usize;
        Ok(output.bytes.0[..len].to_vec())
    }

    /// Writes raw bytes to an SMC key.
    fn write_key_bytes(
        &self,
        key: u32,
        data_size: u32,
        bytes: &[u8],
    ) -> Result<(), SmcWriteError> {
        let mut input = SmcKeyData::default();
        input.key = key;
        input.data8 = SMC_CMD_WRITE_BYTES;
        input.key_info.data_size = data_size;

        let len = (data_size as usize).min(bytes.len()).min(32);
        input.bytes.0[..len].copy_from_slice(&bytes[..len]);

        let mut output = SmcKeyData::default();
        self.smc_call(&input, &mut output)
    }

    /// Sends one SMC command via `IOConnectCallStructMethod`.
    fn smc_call(
        &self,
        input: &SmcKeyData,
        output: &mut SmcKeyData,
    ) -> Result<(), SmcWriteError> {
        let mut output_size = std::mem::size_of::<SmcKeyData>();

        let result = unsafe {
            IOConnectCallStructMethod(
                self.conn,
                KERNEL_INDEX_SMC,
                input as *const _ as *const c_void,
                std::mem::size_of::<SmcKeyData>(),
                output as *mut _ as *mut c_void,
                &mut output_size,
            )
        };

        if result == RETURN_NOT_PRIVILEGED {
            return Err(SmcWriteError::InsufficientPrivileges);
        }
        if result != KERN_SUCCESS {
            return Err(SmcWriteError::CallFailed(result, output.result));
        }

        // Check SMC-level result (separate from IOKit return code).
        match output.result {
            0 => Ok(()),
            132 => {
                let key_bytes = input.key.to_be_bytes();
                let key_str = String::from_utf8_lossy(&key_bytes).to_string();
                Err(SmcWriteError::UnknownKey(key_str))
            }
            134 => Err(SmcWriteError::InsufficientPrivileges),
            code => Err(SmcWriteError::CallFailed(result, code)),
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Opens a new IOKit connection to AppleSMC.
unsafe fn smc_open() -> Result<io_connect_t, SmcWriteError> {
    let matching_dictionary = IOServiceMatching(b"AppleSMC\0".as_ptr());
    let device = IOServiceGetMatchingService(MASTER_PORT_DEFAULT, matching_dictionary);

    if device == MACH_PORT_NULL {
        return Err(SmcWriteError::DeviceNotFound);
    }

    let mut conn: io_connect_t = MACH_PORT_NULL;
    let result = IOServiceOpen(device, mach_task_self(), 0, &mut conn);
    let _ = IOObjectRelease(device);

    if result != KERN_SUCCESS {
        return Err(SmcWriteError::OpenFailed(result));
    }

    eprintln!("[smc_writer] smc_open: conn={conn}");
    Ok(conn)
}

/// Builds a 4-char SMC key for a fan, e.g. `F0Tg` or `F1Md`.
fn fan_key(fan_index: u8, suffix: &[u8; 2]) -> u32 {
    u32::from_be_bytes([b'F', b'0' + fan_index, suffix[0], suffix[1]])
}

/// Encodes a float value into the SMC byte format for the given data type.
fn encode_value(value: f32, data_type: u32, data_size: u32) -> Result<Vec<u8>, SmcWriteError> {
    let type_bytes = data_type.to_be_bytes();

    match &type_bytes {
        b"flt " => Ok(value.to_be_bytes().to_vec()),
        b"fpe2" => {
            let encoded = (value * 4.0).round() as u16;
            Ok(encoded.to_be_bytes().to_vec())
        }
        b"sp78" => {
            let encoded = (value * 256.0).round() as i16;
            Ok(encoded.to_be_bytes().to_vec())
        }
        [b'u', b'i', b'8', b' '] => Ok(vec![value.round() as u8]),
        [b'u', b'i', b'1', b'6'] => {
            let encoded = value.round() as u16;
            Ok(encoded.to_be_bytes().to_vec())
        }
        _ if data_size == 4 => Ok(value.to_be_bytes().to_vec()),
        _ if data_size == 2 => {
            let encoded = (value * 4.0).round() as u16;
            Ok(encoded.to_be_bytes().to_vec())
        }
        _ => Ok(value.to_be_bytes().to_vec()),
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fan_key_generates_correct_bytes() {
        let key = fan_key(0, b"Tg");
        assert_eq!(&key.to_be_bytes(), b"F0Tg");

        let key = fan_key(1, b"Md");
        assert_eq!(&key.to_be_bytes(), b"F1Md");

        let key = fan_key(2, b"Ac");
        assert_eq!(&key.to_be_bytes(), b"F2Ac");
    }

    #[test]
    fn encode_fpe2_value() {
        let bytes = encode_value(2400.0, u32::from_be_bytes(*b"fpe2"), 2).unwrap();
        assert_eq!(bytes, vec![0x25, 0x80]);
    }

    #[test]
    fn encode_flt_value() {
        let bytes = encode_value(2400.0, u32::from_be_bytes(*b"flt "), 4).unwrap();
        assert_eq!(bytes, 2400.0_f32.to_be_bytes().to_vec());
    }

    #[test]
    fn encode_ui8_value() {
        let bytes = encode_value(1.0, u32::from_be_bytes(*b"ui8 "), 1).unwrap();
        assert_eq!(bytes, vec![1]);

        let bytes = encode_value(0.0, u32::from_be_bytes(*b"ui8 "), 1).unwrap();
        assert_eq!(bytes, vec![0]);
    }
}
