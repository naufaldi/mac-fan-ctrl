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
use std::time::Duration;
use thiserror::Error;

use crate::log::{debug_log, warn_log};

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
    #[error("Timed out waiting for Apple Silicon fan control handoff")]
    ModeTransitionTimedOut,
    #[error("Fan mode verification failed: mode {actual} still blocks target writes")]
    ModeVerificationFailed { actual: u8 },
    #[error("Privileged helper is not running")]
    HelperNotRunning,
    #[error("Helper communication error: {0}")]
    HelperError(String),
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
const APPLE_SILICON_AUTO_MODE: u8 = 0;
const APPLE_SILICON_MANUAL_MODE: u8 = 1;
const APPLE_SILICON_SYSTEM_MODE: u8 = 3;
const MODE_POLL_INTERVAL: Duration = Duration::from_millis(100);
const MODE_TRANSITION_RETRY_COUNT: u32 = 100;

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

// ── Public trait for fan control writes ──────────────────────────────────────

/// Abstraction over SMC write operations, enabling mock-based testing of
/// safety-critical fan control logic without hardware access.
pub trait SmcWriteApi: Send + Sync {
    fn set_fan_target_rpm(
        &self,
        fan_index: u8,
        rpm: f32,
        min_rpm: f32,
        max_rpm: f32,
    ) -> Result<(), SmcWriteError>;

    fn set_fan_auto(&self, fan_index: u8) -> Result<(), SmcWriteError>;
    fn lock_fan_control(&self) -> Result<(), SmcWriteError>;
    #[allow(dead_code)]
    fn unlock_fan_control(&self) -> Result<(), SmcWriteError>;
    fn diagnose_fan_control(&self) -> Vec<String>;
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
        // Best-effort: restore all fans to auto and re-lock thermal control
        for i in 0..8u8 {
            let _ = self.set_fan_auto_impl(i);
        }
        let _ = self.lock_fan_control_impl();
        unsafe {
            let _ = IOServiceClose(self.conn);
        }
    }
}

impl SmcWriteApi for SmcWriter {
    fn set_fan_target_rpm(
        &self,
        fan_index: u8,
        rpm: f32,
        min_rpm: f32,
        max_rpm: f32,
    ) -> Result<(), SmcWriteError> {
        self.set_fan_target_rpm_impl(fan_index, rpm, min_rpm, max_rpm)
    }

    fn set_fan_auto(&self, fan_index: u8) -> Result<(), SmcWriteError> {
        self.set_fan_auto_impl(fan_index)
    }

    fn lock_fan_control(&self) -> Result<(), SmcWriteError> {
        self.lock_fan_control_impl()
    }

    fn unlock_fan_control(&self) -> Result<(), SmcWriteError> {
        self.unlock_fan_control_impl()
    }

    fn diagnose_fan_control(&self) -> Vec<String> {
        self.diagnose_fan_control_impl()
    }
}

impl SmcWriter {
    /// Opens a new connection to the AppleSMC kernel service.
    pub fn new() -> Result<Self, SmcWriteError> {
        let conn = unsafe { smc_open() }?;
        debug_log!("[smc_writer] SmcWriter::new() — connection opened (conn={conn})");
        Ok(Self { conn })
    }

    /// Dumps full diagnostic info for fan control debugging.
    /// Does NOT write anything — purely reads SMC state.
    fn diagnose_fan_control_impl(&self) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();

        lines.push("=== SMC Fan Control Diagnostic Report ===".to_string());
        lines.push(format!("Connection handle: {}", self.conn));

        // Check Ftst key
        let ftst_key = u32::from_be_bytes(*b"Ftst");
        match self.read_key_info(ftst_key) {
            Ok(info) => {
                let type_bytes = info.data_type.to_be_bytes();
                let type_str = String::from_utf8_lossy(&type_bytes);
                lines.push(format!("Ftst key: EXISTS (type={type_str} size={})", info.data_size));
                match self.read_key_bytes(ftst_key, info.data_size) {
                    Ok(val) => lines.push(format!("Ftst value: {:?}", val)),
                    Err(e) => lines.push(format!("Ftst read error: {e}")),
                }
            }
            Err(SmcWriteError::UnknownKey(_)) => {
                lines.push("Ftst key: NOT FOUND — this M1 does not have diagnostic unlock key".to_string());
                lines.push("  -> Direct F*Md writes may work (pre-14.4 firmware) or may be blocked".to_string());
            }
            Err(e) => lines.push(format!("Ftst key check error: {e}")),
        }

        // Check FNum (number of fans)
        let fnum_key = u32::from_be_bytes(*b"FNum");
        match self.read_key_info(fnum_key) {
            Ok(info) => match self.read_key_bytes(fnum_key, info.data_size) {
                Ok(val) => {
                    let num_fans = val.first().copied().unwrap_or(0);
                    lines.push(format!("FNum (fan count): {num_fans}"));

                    // Dump each fan's state
                    for i in 0..num_fans {
                        lines.push(format!("--- Fan {i} ---"));

                        // Mode
                        let md_key = fan_key(i, b"Md");
                        match self.read_key_info(md_key) {
                            Ok(md_info) => match self.read_key_bytes(md_key, md_info.data_size) {
                                Ok(md_val) => {
                                    let mode = md_val.first().copied().unwrap_or(255);
                                    let mode_name = match mode {
                                        0 => "Auto",
                                        1 => "Forced/Manual",
                                        3 => "System (thermalmonitord enforced)",
                                        _ => "Unknown",
                                    };
                                    lines.push(format!("  F{i}Md (mode): {mode} = {mode_name}"));
                                }
                                Err(e) => lines.push(format!("  F{i}Md read error: {e}")),
                            },
                            Err(e) => lines.push(format!("  F{i}Md key error: {e}")),
                        }

                        // Actual RPM
                        let ac_key = fan_key(i, b"Ac");
                        match self.read_key_info(ac_key) {
                            Ok(ac_info) => match self.read_key_bytes(ac_key, ac_info.data_size) {
                                Ok(ac_val) => {
                                    let type_bytes = ac_info.data_type.to_be_bytes();
                                    let rpm = decode_rpm(&ac_val, &type_bytes);
                                    lines.push(format!("  F{i}Ac (actual RPM): {rpm:.0} (raw={ac_val:?})"));
                                }
                                Err(e) => lines.push(format!("  F{i}Ac read error: {e}")),
                            },
                            Err(e) => lines.push(format!("  F{i}Ac key error: {e}")),
                        }

                        // Target RPM
                        let tg_key = fan_key(i, b"Tg");
                        match self.read_key_info(tg_key) {
                            Ok(tg_info) => {
                                let type_bytes = tg_info.data_type.to_be_bytes();
                                let type_str = String::from_utf8_lossy(&type_bytes);
                                match self.read_key_bytes(tg_key, tg_info.data_size) {
                                    Ok(tg_val) => {
                                        let rpm = decode_rpm(&tg_val, &type_bytes);
                                        lines.push(format!("  F{i}Tg (target RPM): {rpm:.0} (type={type_str} raw={tg_val:?})"));
                                    }
                                    Err(e) => lines.push(format!("  F{i}Tg read error: {e}")),
                                }
                            }
                            Err(e) => lines.push(format!("  F{i}Tg key error: {e}")),
                        }

                        // Min RPM
                        let mn_key = fan_key(i, b"Mn");
                        match self.read_key_info(mn_key) {
                            Ok(mn_info) => match self.read_key_bytes(mn_key, mn_info.data_size) {
                                Ok(mn_val) => {
                                    let type_bytes = mn_info.data_type.to_be_bytes();
                                    let rpm = decode_rpm(&mn_val, &type_bytes);
                                    lines.push(format!("  F{i}Mn (min RPM): {rpm:.0}"));
                                }
                                Err(e) => lines.push(format!("  F{i}Mn read error: {e}")),
                            },
                            Err(e) => lines.push(format!("  F{i}Mn key error: {e}")),
                        }

                        // Max RPM
                        let mx_key = fan_key(i, b"Mx");
                        match self.read_key_info(mx_key) {
                            Ok(mx_info) => match self.read_key_bytes(mx_key, mx_info.data_size) {
                                Ok(mx_val) => {
                                    let type_bytes = mx_info.data_type.to_be_bytes();
                                    let rpm = decode_rpm(&mx_val, &type_bytes);
                                    lines.push(format!("  F{i}Mx (max RPM): {rpm:.0}"));
                                }
                                Err(e) => lines.push(format!("  F{i}Mx read error: {e}")),
                            },
                            Err(e) => lines.push(format!("  F{i}Mx key error: {e}")),
                        }
                    }
                }
                Err(e) => lines.push(format!("FNum read error: {e}")),
            },
            Err(e) => lines.push(format!("FNum key error: {e}")),
        }

        lines.push("=== End Diagnostic Report ===".to_string());

        // Also print to stderr
        for line in &lines {
            debug_log!("[diag] {line}");
        }

        lines
    }

    // ── Apple Silicon thermal unlock ─────────────────────────────────────

    /// Sets the `Ftst` (force-test) diagnostic flag to inhibit
    /// `thermalmonitord` thermal enforcement, allowing fan mode writes.
    fn unlock_fan_control_impl(&self) -> Result<(), SmcWriteError> {
        let key = u32::from_be_bytes(*b"Ftst");
        debug_log!("[smc_writer] unlock_fan_control: reading Ftst key info...");
        let key_info = self.read_key_info(key)?;
        let type_bytes = key_info.data_type.to_be_bytes();
        let type_str = String::from_utf8_lossy(&type_bytes);
        debug_log!(
            "[smc_writer] unlock_fan_control: Ftst key found — type={type_str} size={}",
            key_info.data_size
        );

        // Read current Ftst value before writing
        match self.read_key_bytes(key, key_info.data_size) {
            Ok(current) => debug_log!(
                "[smc_writer] unlock_fan_control: Ftst current value={:?}",
                current
            ),
            Err(e) => debug_log!(
                "[smc_writer] unlock_fan_control: could not read Ftst current value: {e}"
            ),
        }

        debug_log!("[smc_writer] unlock_fan_control: writing Ftst=1");
        let result = self.write_key_bytes(key, key_info.data_size, &[1]);
        match &result {
            Ok(()) => {
                // Verify the write took effect
                match self.read_key_bytes(key, key_info.data_size) {
                    Ok(readback) => debug_log!(
                        "[smc_writer] unlock_fan_control: Ftst readback after write={:?}",
                        readback
                    ),
                    Err(e) => debug_log!(
                        "[smc_writer] unlock_fan_control: Ftst readback failed: {e}"
                    ),
                }
            }
            Err(e) => debug_log!("[smc_writer] unlock_fan_control: write FAILED: {e}"),
        }
        result
    }

    /// Clears the `Ftst` flag, re-enabling thermal enforcement.
    fn lock_fan_control_impl(&self) -> Result<(), SmcWriteError> {
        let key = u32::from_be_bytes(*b"Ftst");
        let key_info = self.read_key_info(key)?;
        debug_log!("[smc_writer] lock_fan_control: writing Ftst=0");
        self.write_key_bytes(key, key_info.data_size, &[0])
    }

    // ── Fan control helpers ──────────────────────────────────────────────

    /// Sets fan to forced mode and writes a target RPM.
    ///
    /// Strategy (adapts to chip generation):
    ///   1. Try `Ftst=1` unlock (M3/M4 need it; gracefully skipped if key absent)
    ///   2. Write `F{n}Md=1` (forced mode)
    ///   3. Write `F{n}Tg=<rpm>`
    fn set_fan_target_rpm_impl(
        &self,
        fan_index: u8,
        rpm: f32,
        min_rpm: f32,
        max_rpm: f32,
    ) -> Result<(), SmcWriteError> {
        debug_log!("[smc_writer] set_fan_target_rpm: fan={fan_index} rpm={rpm} bounds=[{min_rpm}, {max_rpm}]");
        if rpm < min_rpm || rpm > max_rpm {
            return Err(SmcWriteError::InvalidRpm {
                min: min_rpm,
                max: max_rpm,
                requested: rpm,
            });
        }

        // Step 1: Unlock thermal enforcement (best-effort — key may not exist on M1)
        match self.unlock_fan_control_impl() {
            Ok(()) => debug_log!("[smc_writer] Ftst unlock OK"),
            Err(SmcWriteError::UnknownKey(_)) => {
                debug_log!("[smc_writer] Ftst key not present (M1) — skipping unlock");
            }
            Err(e) => {
                debug_log!("[smc_writer] Ftst unlock failed: {e} — continuing anyway");
            }
        }

        self.wait_for_system_mode_handoff(fan_index)?;

        // Step 2: Set forced mode
        debug_log!("[smc_writer] Setting F{fan_index}Md=1 (forced)");
        self.set_fan_mode(fan_index, true)?;
        self.verify_mode_allows_target_write(fan_index)?;

        // Step 3: Write target RPM
        let key = fan_key(fan_index, b"Tg");
        let key_info = self.read_key_info(key)?;
        let type_bytes = key_info.data_type.to_be_bytes();
        let type_str = String::from_utf8_lossy(&type_bytes);
        debug_log!(
            "[smc_writer] F{fan_index}Tg type={type_str} size={}",
            key_info.data_size
        );
        let bytes = encode_value(rpm, key_info.data_type, key_info.data_size)?;
        self.write_key_bytes(key, key_info.data_size, &bytes)?;

        // Verify write persisted after SMC settles (immediate reads may show stale data)
        std::thread::sleep(Duration::from_millis(100));
        match self.read_key_bytes(key, key_info.data_size) {
            Ok(readback) => {
                let readback_rpm = decode_rpm(&readback, &type_bytes);
                let diff = (readback_rpm - rpm).abs();
                if diff > 50.0 {
                    debug_log!(
                        "[smc_writer] WARNING: F{fan_index}Tg write did not persist — wrote={rpm:.0} readback={readback_rpm:.0}"
                    );
                } else {
                    debug_log!("[smc_writer] F{fan_index}Tg verified: {readback_rpm:.0} RPM");
                }
            }
            Err(e) => debug_log!("[smc_writer] F{fan_index}Tg readback failed: {e}"),
        }

        debug_log!("[smc_writer] set_fan_target_rpm: done fan={fan_index} rpm={rpm}");
        Ok(())
    }

    /// Returns fan to automatic (system-controlled) mode.
    /// Caller is responsible for calling `lock_fan_control()` when
    /// no fans remain in forced mode.
    fn set_fan_auto_impl(&self, fan_index: u8) -> Result<(), SmcWriteError> {
        debug_log!("[smc_writer] set_fan_auto: fan={fan_index}");
        self.set_fan_mode(fan_index, false)
    }

    /// Sets the fan mode flag: `false` = Auto, `true` = Forced.
    fn set_fan_mode(&self, fan_index: u8, forced: bool) -> Result<(), SmcWriteError> {
        let key = fan_key(fan_index, b"Md");
        let key_info = self.read_key_info(key)?;
        let value: u8 = if forced {
            APPLE_SILICON_MANUAL_MODE
        } else {
            APPLE_SILICON_AUTO_MODE
        };
        self.write_key_bytes(key, key_info.data_size, &[value])
    }

    fn wait_for_system_mode_handoff(&self, fan_index: u8) -> Result<(), SmcWriteError> {
        debug_log!("[smc_writer] wait_for_system_mode_handoff: fan={fan_index} — polling mode (max {MODE_TRANSITION_RETRY_COUNT} x {}ms)...",
            MODE_POLL_INTERVAL.as_millis());
        let initial_mode = self.read_fan_mode(fan_index)?;
        debug_log!("[smc_writer] wait_for_system_mode_handoff: initial mode={initial_mode} (0=Auto, 1=Forced, 3=System)");

        let mut poll_count: u32 = 0;
        let result = wait_for_system_mode_handoff(
            || {
                let mode = self.read_fan_mode(fan_index)?;
                poll_count += 1;
                if poll_count <= 5 || poll_count % 20 == 0 {
                    debug_log!("[smc_writer] wait_for_system_mode_handoff: poll #{poll_count} mode={mode}");
                }
                Ok(mode)
            },
            std::thread::sleep,
        );

        match &result {
            Ok(()) => {
                let final_mode = self.read_fan_mode(fan_index).unwrap_or(255);
                debug_log!("[smc_writer] wait_for_system_mode_handoff: OK after {poll_count} polls, final mode={final_mode}");
            }
            Err(e) => debug_log!("[smc_writer] wait_for_system_mode_handoff: FAILED after {poll_count} polls: {e}"),
        }
        result
    }

    fn read_fan_mode(&self, fan_index: u8) -> Result<u8, SmcWriteError> {
        let key = fan_key(fan_index, b"Md");
        let key_info = self.read_key_info(key)?;
        let bytes = self.read_key_bytes(key, key_info.data_size)?;
        bytes
            .first()
            .copied()
            .ok_or(SmcWriteError::InvalidFanId(fan_index))
    }

    fn verify_mode_allows_target_write(&self, fan_index: u8) -> Result<(), SmcWriteError> {
        let actual_mode = self.read_fan_mode(fan_index)?;
        debug_log!("[smc_writer] verify_mode: fan={fan_index} actual_mode={actual_mode} (need 0 or 1, reject 3)");
        validate_mode_allows_target_write(actual_mode)
    }

    // ── Low-level SMC operations ─────────────────────────────────────────

    /// Reads key info (data type + size) for a given 4-char SMC key.
    fn read_key_info(&self, key: u32) -> Result<SmcKeyDataKeyInfo, SmcWriteError> {
        let input = SmcKeyData {
            key,
            data8: SMC_CMD_READ_KEYINFO,
            ..Default::default()
        };

        let mut output = SmcKeyData::default();
        self.smc_call(&input, &mut output)?;

        Ok(output.key_info)
    }

    /// Reads raw bytes from an SMC key.
    #[allow(dead_code)]
    fn read_key_bytes(&self, key: u32, data_size: u32) -> Result<Vec<u8>, SmcWriteError> {
        let input = SmcKeyData {
            key,
            data8: SMC_CMD_READ_BYTES,
            key_info: SmcKeyDataKeyInfo { data_size, ..Default::default() },
            ..Default::default()
        };

        let mut output = SmcKeyData::default();
        self.smc_call(&input, &mut output)?;

        let len = data_size as usize;
        Ok(output.bytes.0[..len].to_vec())
    }

    /// Writes raw bytes to an SMC key.
    fn write_key_bytes(&self, key: u32, data_size: u32, bytes: &[u8]) -> Result<(), SmcWriteError> {
        let input_base = SmcKeyData {
            key,
            data8: SMC_CMD_WRITE_BYTES,
            key_info: SmcKeyDataKeyInfo { data_size, ..Default::default() },
            ..Default::default()
        };
        let mut input = input_base;

        let len = (data_size as usize).min(bytes.len()).min(32);
        input.bytes.0[..len].copy_from_slice(&bytes[..len]);

        let mut output = SmcKeyData::default();
        self.smc_call(&input, &mut output)
    }

    /// Sends one SMC command via `IOConnectCallStructMethod`.
    fn smc_call(&self, input: &SmcKeyData, output: &mut SmcKeyData) -> Result<(), SmcWriteError> {
        let key_bytes = input.key.to_be_bytes();
        let key_str = String::from_utf8_lossy(&key_bytes);
        let cmd_name = match input.data8 {
            SMC_CMD_READ_KEYINFO => "READ_KEYINFO",
            SMC_CMD_READ_BYTES => "READ_BYTES",
            SMC_CMD_WRITE_BYTES => "WRITE_BYTES",
            other => {
                debug_log!("[smc_writer] smc_call: key={key_str} cmd=UNKNOWN({other})");
                "UNKNOWN"
            }
        };

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
            warn_log!("[smc_writer] smc_call: key={key_str} cmd={cmd_name} -> IOKit RETURN_NOT_PRIVILEGED ({result:#010x})");
            return Err(SmcWriteError::InsufficientPrivileges);
        }
        if result != KERN_SUCCESS {
            debug_log!("[smc_writer] smc_call: key={key_str} cmd={cmd_name} -> IOKit FAILED result={result:#010x} smc_result={}", output.result);
            return Err(SmcWriteError::CallFailed(result, output.result));
        }

        // Check SMC-level result (separate from IOKit return code).
        match output.result {
            0 => {
                // Only log writes at this level to avoid flooding
                if input.data8 == SMC_CMD_WRITE_BYTES {
                    let data_len = input.key_info.data_size as usize;
                    debug_log!(
                        "[smc_writer] smc_call: key={key_str} cmd={cmd_name} -> OK (wrote {} bytes: {:?})",
                        data_len,
                        &input.bytes.0[..data_len.min(8)]
                    );
                }
                Ok(())
            }
            132 => {
                debug_log!("[smc_writer] smc_call: key={key_str} cmd={cmd_name} -> SMC result=132 (UNKNOWN KEY)");
                Err(SmcWriteError::UnknownKey(key_str.to_string()))
            }
            134 => {
                debug_log!("[smc_writer] smc_call: key={key_str} cmd={cmd_name} -> SMC result=134 (INSUFFICIENT PRIVILEGES)");
                Err(SmcWriteError::InsufficientPrivileges)
            }
            code => {
                debug_log!("[smc_writer] smc_call: key={key_str} cmd={cmd_name} -> SMC result={code} (UNEXPECTED)");
                Err(SmcWriteError::CallFailed(result, code))
            }
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Opens a new IOKit connection to AppleSMC.
unsafe fn smc_open() -> Result<io_connect_t, SmcWriteError> {
    let matching_dictionary = IOServiceMatching(c"AppleSMC".as_ptr().cast());
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

    warn_log!("[smc_writer] smc_open: conn={conn}");
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
        // `flt ` uses native endian on Apple Silicon (matches macsmc crate behavior).
        // Integer-based types (fpe2, sp78, ui*) remain big-endian per SMC protocol.
        b"flt " => Ok(value.to_ne_bytes().to_vec()),
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
        _ if data_size == 4 => Ok(value.to_ne_bytes().to_vec()),
        _ if data_size == 2 => {
            let encoded = (value * 4.0).round() as u16;
            Ok(encoded.to_be_bytes().to_vec())
        }
        _ => Ok(value.to_ne_bytes().to_vec()),
    }
}

/// Decodes raw SMC bytes to RPM value based on data type.
/// `flt ` uses native endian (matching macsmc crate / Apple Silicon behavior).
/// Integer-based types (fpe2, sp78) remain big-endian per SMC protocol.
fn decode_rpm(bytes: &[u8], type_bytes: &[u8; 4]) -> f32 {
    match type_bytes {
        b"flt " if bytes.len() >= 4 => {
            f32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
        }
        b"fpe2" if bytes.len() >= 2 => {
            let raw = u16::from_be_bytes([bytes[0], bytes[1]]);
            raw as f32 / 4.0
        }
        b"sp78" if bytes.len() >= 2 => {
            let raw = i16::from_be_bytes([bytes[0], bytes[1]]);
            raw as f32 / 256.0
        }
        _ if bytes.len() >= 4 => {
            f32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
        }
        _ if bytes.len() >= 2 => {
            let raw = u16::from_be_bytes([bytes[0], bytes[1]]);
            raw as f32 / 4.0
        }
        _ => 0.0,
    }
}

fn wait_for_system_mode_handoff<FRead, FSleep>(
    mut read_mode: FRead,
    mut sleep_fn: FSleep,
) -> Result<(), SmcWriteError>
where
    FRead: FnMut() -> Result<u8, SmcWriteError>,
    FSleep: FnMut(Duration),
{
    for _attempt in 0..MODE_TRANSITION_RETRY_COUNT {
        if read_mode()? != APPLE_SILICON_SYSTEM_MODE {
            return Ok(());
        }

        sleep_fn(MODE_POLL_INTERVAL);
    }

    if read_mode()? == APPLE_SILICON_SYSTEM_MODE {
        return Err(SmcWriteError::ModeTransitionTimedOut);
    }

    Ok(())
}

fn validate_mode_allows_target_write(actual_mode: u8) -> Result<(), SmcWriteError> {
    match actual_mode {
        APPLE_SILICON_AUTO_MODE | APPLE_SILICON_MANUAL_MODE => Ok(()),
        _ => Err(SmcWriteError::ModeVerificationFailed {
            actual: actual_mode,
        }),
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

// ── Test mock ────────────────────────────────────────────────────────────────

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::cell::RefCell;

    #[derive(Debug, Clone, PartialEq)]
    pub enum MockSmcCall {
        SetFanTargetRpm { fan_index: u8, rpm: f32 },
        SetFanAuto { fan_index: u8 },
        LockFanControl,
        UnlockFanControl,
        DiagnoseFanControl,
    }

    pub struct MockSmcWriter {
        pub calls: RefCell<Vec<MockSmcCall>>,
        pub should_fail: bool,
    }

    // Safety: MockSmcWriter is only used in single-threaded test contexts.
    unsafe impl Send for MockSmcWriter {}
    unsafe impl Sync for MockSmcWriter {}

    impl MockSmcWriter {
        pub fn new() -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
                should_fail: false,
            }
        }

        pub fn failing() -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
                should_fail: true,
            }
        }
    }

    impl SmcWriteApi for MockSmcWriter {
        fn set_fan_target_rpm(
            &self,
            fan_index: u8,
            rpm: f32,
            _min_rpm: f32,
            _max_rpm: f32,
        ) -> Result<(), SmcWriteError> {
            self.calls
                .borrow_mut()
                .push(MockSmcCall::SetFanTargetRpm { fan_index, rpm });
            if self.should_fail {
                Err(SmcWriteError::InsufficientPrivileges)
            } else {
                Ok(())
            }
        }

        fn set_fan_auto(&self, fan_index: u8) -> Result<(), SmcWriteError> {
            self.calls
                .borrow_mut()
                .push(MockSmcCall::SetFanAuto { fan_index });
            if self.should_fail {
                Err(SmcWriteError::InsufficientPrivileges)
            } else {
                Ok(())
            }
        }

        fn lock_fan_control(&self) -> Result<(), SmcWriteError> {
            self.calls.borrow_mut().push(MockSmcCall::LockFanControl);
            if self.should_fail {
                Err(SmcWriteError::InsufficientPrivileges)
            } else {
                Ok(())
            }
        }

        fn unlock_fan_control(&self) -> Result<(), SmcWriteError> {
            self.calls
                .borrow_mut()
                .push(MockSmcCall::UnlockFanControl);
            if self.should_fail {
                Err(SmcWriteError::InsufficientPrivileges)
            } else {
                Ok(())
            }
        }

        fn diagnose_fan_control(&self) -> Vec<String> {
            self.calls
                .borrow_mut()
                .push(MockSmcCall::DiagnoseFanControl);
            vec!["mock diagnostic".to_string()]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

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
        assert_eq!(bytes, 2400.0_f32.to_ne_bytes().to_vec());
    }

    #[test]
    fn encode_ui8_value() {
        let bytes = encode_value(1.0, u32::from_be_bytes(*b"ui8 "), 1).unwrap();
        assert_eq!(bytes, vec![1]);

        let bytes = encode_value(0.0, u32::from_be_bytes(*b"ui8 "), 1).unwrap();
        assert_eq!(bytes, vec![0]);
    }

    #[test]
    fn wait_for_system_mode_handoff_retries_until_auto_mode() {
        let mut observed_sleeps: Vec<Duration> = Vec::new();
        let mut responses = vec![3_u8, 3_u8, 0_u8].into_iter();

        let result = wait_for_system_mode_handoff(
            || responses.next().ok_or(SmcWriteError::InvalidFanId(0)),
            |duration| observed_sleeps.push(duration),
        );

        assert!(result.is_ok());
        assert_eq!(
            observed_sleeps,
            vec![MODE_POLL_INTERVAL, MODE_POLL_INTERVAL]
        );
    }

    #[test]
    fn wait_for_system_mode_handoff_times_out_when_system_mode_persists() {
        let mut observed_sleeps: Vec<Duration> = Vec::new();

        let result = wait_for_system_mode_handoff(
            || Ok(APPLE_SILICON_SYSTEM_MODE),
            |duration| observed_sleeps.push(duration),
        );

        assert!(matches!(result, Err(SmcWriteError::ModeTransitionTimedOut)));
        assert_eq!(observed_sleeps.len(), MODE_TRANSITION_RETRY_COUNT as usize);
        assert!(observed_sleeps
            .iter()
            .all(|duration| *duration == MODE_POLL_INTERVAL));
    }

    #[test]
    fn validate_mode_allows_target_write_accepts_auto_readback() {
        let result = validate_mode_allows_target_write(APPLE_SILICON_AUTO_MODE);

        assert!(result.is_ok());
    }

    #[test]
    fn validate_mode_allows_target_write_accepts_manual_readback() {
        let result = validate_mode_allows_target_write(APPLE_SILICON_MANUAL_MODE);

        assert!(result.is_ok());
    }

    #[test]
    fn validate_mode_allows_target_write_rejects_system_mode() {
        let result = validate_mode_allows_target_write(APPLE_SILICON_SYSTEM_MODE);

        assert!(matches!(
            result,
            Err(SmcWriteError::ModeVerificationFailed {
                actual: APPLE_SILICON_SYSTEM_MODE,
            })
        ));
    }
}
