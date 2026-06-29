//! Bundled privileged helper binary discovery and validation.

use std::path::{Path, PathBuf};

const HELPER_BASENAME: &str = "fanguard-helper";

/// Collects candidate helper paths from the app bundle and dev target directories.
pub fn helper_binary_candidates(exe_path: &Path) -> Vec<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    let mut seen: Vec<PathBuf> = Vec::new();

    let mut push_unique = |path: PathBuf| {
        if path.exists() && !seen.iter().any(|existing| existing == &path) {
            seen.push(path.clone());
            candidates.push(path);
        }
    };

    if let Some(mac_os_dir) = exe_path.parent() {
        push_unique(mac_os_dir.join(HELPER_BASENAME));
        if let Ok(entries) = std::fs::read_dir(mac_os_dir) {
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                let name = file_name.to_string_lossy().into_owned();
                if name.starts_with(HELPER_BASENAME) {
                    push_unique(entry.path());
                }
            }
        }
    }

    let target_dir = exe_path.parent().unwrap_or(exe_path);
    push_unique(target_dir.join(HELPER_BASENAME));

    if let Ok(entries) = std::fs::read_dir("src-tauri") {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy().into_owned();
            if name.starts_with(HELPER_BASENAME) {
                push_unique(entry.path());
            }
        }
    }

    candidates
}

/// Returns true when the path looks like a real Mach-O binary, not a shell stub.
pub fn is_valid_helper_binary(path: &Path) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };

    if bytes.starts_with(b"#!") {
        return false;
    }

    if bytes.len() < 4 {
        return false;
    }

    let magic = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    matches!(magic, 0xFEED_FACF | 0xFEED_FACE | 0xCAFE_BABE | 0xCEFA_EDFE)
        || bytes.starts_with(b"\x7fELF")
}

/// Finds the first valid helper binary among known bundle/dev locations.
pub fn find_helper_binary(exe_path: &Path) -> Result<PathBuf, String> {
    let candidates = helper_binary_candidates(exe_path);
    let tried: Vec<String> = candidates
        .iter()
        .map(|path| path.display().to_string())
        .collect();

    candidates
        .into_iter()
        .find(|path| is_valid_helper_binary(path))
        .ok_or_else(|| {
            if tried.is_empty() {
                "Helper binary not found. Build it with: cargo build --bin fanguard-helper"
                    .to_string()
            } else {
                format!(
                    "Helper binary not found or invalid (shell stub?). Tried: {}",
                    tried.join(", ")
                )
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn rejects_shell_stub_helper() {
        let mut temp = NamedTempFile::new().expect("temp file");
        writeln!(temp, "#!/bin/sh").expect("write stub");
        temp.flush().expect("flush stub");

        assert!(!is_valid_helper_binary(temp.path()));
    }

    #[test]
    fn accepts_mach_o_fat_header() {
        let mut temp = NamedTempFile::new().expect("temp file");
        temp.write_all(&0xCAFE_BABEu32.to_be_bytes())
            .expect("write magic");
        temp.flush().expect("flush mach-o");

        assert!(is_valid_helper_binary(temp.path()));
    }
}
