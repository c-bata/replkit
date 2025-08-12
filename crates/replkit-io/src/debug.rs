//! Debug logging utilities for replkit
//!
//! This module provides logging functionality similar to go-prompt,
//! allowing debug output to be written to a file when enabled.

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Mutex, Once};

static INIT: Once = Once::new();
static LOG_FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);

const ENV_ENABLE_LOG: &str = "REPLKIT_DEBUG";

fn init_logger() {
    INIT.call_once(|| {
        if let Ok(val) = std::env::var(ENV_ENABLE_LOG) {
            if val == "true" || val == "1" {
                let log_path = if std::path::Path::new("tmp").exists() {
                    "tmp/replkit-debug.log"
                } else {
                    "/tmp/replkit-debug.log"
                };

                match OpenOptions::new().create(true).append(true).open(log_path) {
                    Ok(file) => {
                        *LOG_FILE.lock().unwrap() = Some(file);
                        eprintln!("replkit debug log enabled: {log_path}");
                    }
                    Err(e) => {
                        eprintln!("Failed to open debug log file {log_path}: {e}");
                    }
                }
            }
        }
    });
}

pub fn write_log(msg: &str) {
    init_logger();

    if let Ok(mut log_file_guard) = LOG_FILE.lock() {
        if let Some(ref mut file) = *log_file_guard {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            if let Err(e) = writeln!(file, "[{timestamp}] {msg}") {
                eprintln!("Failed to write to debug log: {e}");
            } else {
                let _ = file.flush(); // Immediately flush
            }
        }
    }
}

#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            $crate::debug::write_log(&format!($($arg)*));
        }
    };
}
