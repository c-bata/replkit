//! Cross-platform console input abstraction and backends.
//!
//! Defines the `ConsoleInput` trait and provides platform implementations:
//! - UnixVtConsoleInput (POSIX/VT)
//! - WindowsVtConsoleInput (VT in Windows Terminal/PowerShell) [stubbed here]
//! - WindowsLegacyConsoleInput (cmd.exe events) [stubbed here]

use std::fmt;
use std::io;

pub use prompt_core::{KeyEvent, KeyParser};

/// Result type for console input operations.
pub type ConsoleResult<T> = Result<T, ConsoleError>;

/// Errors that can occur in console input handling.
#[derive(Debug)]
pub enum ConsoleError {
    Io(io::Error),
    UnsupportedFeature(&'static str),
    AlreadyRunning,
    NotRunning,
}

impl fmt::Display for ConsoleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConsoleError::Io(e) => write!(f, "I/O error: {}", e),
            ConsoleError::UnsupportedFeature(s) => write!(f, "Unsupported feature: {}", s),
            ConsoleError::AlreadyRunning => write!(f, "Event loop already running"),
            ConsoleError::NotRunning => write!(f, "Event loop is not running"),
        }
    }
}

impl From<io::Error> for ConsoleError {
    fn from(e: io::Error) -> Self { ConsoleError::Io(e) }
}

/// Cross-platform console input interface.
pub trait ConsoleInput: Send {
    /// ターミナルをraw modeにする（元に戻すハンドルはDropに実装）
    fn enable_raw_mode(&mut self) -> ConsoleResult<()>;

    /// 現在のウィンドウサイズ（columns, rows）を取得
    fn get_window_size(&self) -> ConsoleResult<(u16, u16)>;

    /// ウィンドウサイズが変更されたらコールバックを呼ぶ
    fn set_resize_callback(&mut self, callback: Box<dyn FnMut(u16, u16) + Send>);

    /// キー入力があればイベントを通知
    fn set_key_event_callback(&mut self, callback: Box<dyn FnMut(KeyEvent) + Send>);

    /// イベントループを開始する
    fn start_event_loop(&mut self) -> ConsoleResult<()>;

    /// イベントループを停止する
    fn stop_event_loop(&mut self) -> ConsoleResult<()>;
}

#[cfg(unix)]
mod unix;

#[cfg(windows)]
mod windows;

#[cfg(unix)]
pub use unix::UnixVtConsoleInput;

#[cfg(windows)]
pub use windows::{WindowsLegacyConsoleInput, WindowsVtConsoleInput};
