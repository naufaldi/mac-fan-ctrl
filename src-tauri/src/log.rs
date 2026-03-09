/// Debug-only logging — compiled out in release builds.
macro_rules! debug_log {
    ($($arg:tt)*) => {{
        #[cfg(debug_assertions)]
        {
            eprintln!($($arg)*)
        }
        #[cfg(not(debug_assertions))]
        {
            // Silence unused-variable warnings for format args in release builds.
            let _ = format_args!($($arg)*);
        }
    }};
}

/// Always-on logging for safety-critical paths.
macro_rules! warn_log {
    ($($arg:tt)*) => {{
        eprintln!($($arg)*)
    }};
}

pub(crate) use debug_log;
pub(crate) use warn_log;
