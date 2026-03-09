//! Power source detection via IOKit IOPSCopyPowerSourcesInfo.
//!
//! Detects whether the Mac is running on AC power or battery,
//! used for auto-switching fan presets based on power source.

use serde::{Deserialize, Serialize};

use crate::log::debug_log;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerSource {
    Ac,
    Battery,
    Unknown,
}

impl std::fmt::Display for PowerSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PowerSource::Ac => write!(f, "AC"),
            PowerSource::Battery => write!(f, "Battery"),
            PowerSource::Unknown => write!(f, "Unknown"),
        }
    }
}

// FFI declarations for IOKit power source APIs
extern "C" {
    fn IOPSCopyPowerSourcesInfo() -> core_foundation::base::CFTypeRef;
    fn IOPSCopyPowerSourcesList(blob: core_foundation::base::CFTypeRef)
        -> core_foundation::array::CFArrayRef;
    fn IOPSGetPowerSourceDescription(
        blob: core_foundation::base::CFTypeRef,
        ps: *const std::ffi::c_void,
    ) -> core_foundation::dictionary::CFDictionaryRef;
}

pub fn current_power_source() -> PowerSource {
    unsafe {
        let info = IOPSCopyPowerSourcesInfo();
        if info.is_null() {
            debug_log!("[power_monitor] IOPSCopyPowerSourcesInfo returned null");
            return PowerSource::Unknown;
        }

        let sources = IOPSCopyPowerSourcesList(info);
        if sources.is_null() {
            core_foundation::base::CFRelease(info);
            debug_log!("[power_monitor] IOPSCopyPowerSourcesList returned null");
            return PowerSource::Unknown;
        }

        let count = core_foundation::array::CFArrayGetCount(sources);
        if count == 0 {
            // Desktop Macs with no battery report 0 sources — treat as AC
            core_foundation::base::CFRelease(sources as core_foundation::base::CFTypeRef);
            core_foundation::base::CFRelease(info);
            return PowerSource::Ac;
        }

        // Check the first power source
        let ps = core_foundation::array::CFArrayGetValueAtIndex(sources, 0);
        let desc = IOPSGetPowerSourceDescription(info, ps);

        let result = if desc.is_null() {
            PowerSource::Unknown
        } else {
            read_power_source_type(desc)
        };

        core_foundation::base::CFRelease(sources as core_foundation::base::CFTypeRef);
        core_foundation::base::CFRelease(info);

        result
    }
}

unsafe fn read_power_source_type(
    desc: core_foundation::dictionary::CFDictionaryRef,
) -> PowerSource {
    use core_foundation::base::TCFType;
    use core_foundation::string::CFString;

    let key = CFString::new("Power Source State");
    let mut value: *const std::ffi::c_void = std::ptr::null();
    let found = core_foundation::dictionary::CFDictionaryGetValueIfPresent(
        desc,
        key.as_concrete_TypeRef() as *const std::ffi::c_void,
        &mut value,
    );

    if found == 0 || value.is_null() {
        return PowerSource::Unknown;
    }

    let cf_str = CFString::wrap_under_get_rule(value as core_foundation::string::CFStringRef);
    let state = cf_str.to_string();

    match state.as_str() {
        "AC Power" => PowerSource::Ac,
        "Battery Power" => PowerSource::Battery,
        other => {
            debug_log!("[power_monitor] Unknown power source state: {other}");
            PowerSource::Unknown
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_power_source_returns_valid_variant() {
        let source = current_power_source();
        // On any Mac, this should return a valid variant (not panic)
        match source {
            PowerSource::Ac | PowerSource::Battery | PowerSource::Unknown => {}
        }
    }

    #[test]
    fn power_source_display() {
        assert_eq!(PowerSource::Ac.to_string(), "AC");
        assert_eq!(PowerSource::Battery.to_string(), "Battery");
        assert_eq!(PowerSource::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn power_source_serialization() {
        let json = serde_json::to_string(&PowerSource::Ac).expect("serialize");
        assert_eq!(json, "\"ac\"");
        let parsed: PowerSource = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, PowerSource::Ac);
    }
}
