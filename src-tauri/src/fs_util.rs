//! Filesystem utilities for config file persistence.

use std::path::Path;

/// When running as root (e.g. via `sudo`), config files are created with
/// root ownership, making them unwritable when the app later runs as a
/// normal user. Detect `SUDO_UID`/`SUDO_GID` and chown back to the real user.
pub fn fix_ownership_if_root(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;

        let is_root = unsafe { libc::geteuid() } == 0;
        if !is_root {
            return;
        }

        let real_uid: u32 = std::env::var("SUDO_UID")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let real_gid: u32 = std::env::var("SUDO_GID")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        if real_uid == 0 {
            return;
        }

        let needs_chown = std::fs::metadata(path)
            .map(|m| m.uid() != real_uid)
            .unwrap_or(false);

        if needs_chown {
            let c_path = std::ffi::CString::new(path.to_string_lossy().as_bytes()).ok();
            if let Some(p) = c_path {
                unsafe { libc::chown(p.as_ptr(), real_uid, real_gid) };
            }
        }
    }
}
