//! WASM bridge console implementations
//!
//! This module provides console I/O implementations for WASM environments
//! where direct terminal access is not available. Instead, it uses a bridge
//! pattern to communicate with the host environment through serialization.

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

use replkit_core::KeyEvent;
use crate::{ConsoleInput, ConsoleOutput, ConsoleResult, ConsoleError, RawModeGuard,
           ConsoleCapabilities, OutputCapabilities, BackendType, TextStyle, ClearType};

/// WASM bridge console input implementation
pub struct WasmBridgeConsoleInput {
    running: Arc<AtomicBool>,
    resize_callback: Arc<Mutex<Option<Box<dyn FnMut(u16, u16) + Send>>>>,
    key_callback: Arc<Mutex<Option<Box<dyn FnMut(KeyEvent) + Send>>>>,
}

impl WasmBridgeConsoleInput {
    pub fn new() -> std::io::Result<Self> {
        Ok(Self {
            running: Arc::new(AtomicBool::new(false)),
            resize_callback: Arc::new(Mutex::new(None)),
            key_callback: Arc::new(Mutex::new(None)),
        })
    }
    
    /// Receive a message from the host environment
    /// This would be called by WASM-exported functions
    pub fn receive_message(&self, message: &str) -> Result<(), ConsoleError> {
        // In a full implementation, this would deserialize messages from the host
        // and invoke appropriate callbacks
        match message {
            msg if msg.starts_with("key:") => {
                // Parse key event from message
                // For now, just a placeholder
                Ok(())
            }
            msg if msg.starts_with("resize:") => {
                // Parse resize event from message
                // For now, just a placeholder
                Ok(())
            }
            _ => Err(ConsoleError::WasmBridgeError(
                format!("Unknown message type: {}", message)
            ))
        }
    }
}

impl ConsoleInput for WasmBridgeConsoleInput {
    fn enable_raw_mode(&self) -> Result<RawModeGuard, ConsoleError> {
        // In WASM, raw mode is handled by the host environment
        let restore_fn = || {
            // Send message to host to disable raw mode
            // This is a placeholder - real implementation would use wasm-bindgen
        };
        Ok(RawModeGuard::new(restore_fn, "WASM Bridge".to_string()))
    }
    
    fn get_window_size(&self) -> ConsoleResult<(u16, u16)> {
        // In a full implementation, this would query the host environment
        // For now, return a default size
        Ok((80, 24))
    }
    
    fn start_event_loop(&self) -> ConsoleResult<()> {
        if self.running.swap(true, Ordering::Relaxed) {
            return Err(ConsoleError::EventLoopError(crate::EventLoopError::AlreadyRunning));
        }
        
        // In WASM, the event loop is managed by the host environment
        // We just mark ourselves as running
        Ok(())
    }
    
    fn stop_event_loop(&self) -> ConsoleResult<()> {
        if !self.running.swap(false, Ordering::Relaxed) {
            return Err(ConsoleError::EventLoopError(crate::EventLoopError::NotRunning));
        }
        Ok(())
    }
    
    fn on_window_resize(&self, callback: Box<dyn FnMut(u16, u16) + Send>) {
        if let Ok(mut cb) = self.resize_callback.lock() {
            *cb = Some(callback);
        }
    }
    
    fn on_key_pressed(&self, callback: Box<dyn FnMut(KeyEvent) + Send>) {
        if let Ok(mut cb) = self.key_callback.lock() {
            *cb = Some(callback);
        }
    }
    
    fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }
    
    fn get_capabilities(&self) -> ConsoleCapabilities {
        ConsoleCapabilities {
            supports_raw_mode: true,
            supports_resize_events: true,
            supports_bracketed_paste: false,
            supports_mouse_events: false,
            supports_unicode: true,
            platform_name: "WASM Bridge".to_string(),
            backend_type: BackendType::WasmBridge,
        }
    }
}

/// WASM bridge console output implementation
pub struct WasmBridgeConsoleOutput {
    cursor_position: Arc<Mutex<(u16, u16)>>,
    current_style: Arc<Mutex<TextStyle>>,
}

impl WasmBridgeConsoleOutput {
    pub fn new() -> std::io::Result<Self> {
        Ok(Self {
            cursor_position: Arc::Mutex::new((0, 0)),
            current_style: Arc::Mutex::new(TextStyle::default()),
        })
    }
    
    /// Send a message to the host environment
    /// This would be implemented using wasm-bindgen in a full implementation
    fn send_message(&self, message: &str) -> ConsoleResult<()> {
        // Placeholder - real implementation would use wasm-bindgen to call host functions
        #[cfg(target_arch = "wasm32")]
        {
            // In a real implementation, this would be something like:
            // web_sys::console::log_1(&message.into());
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            // For testing on non-WASM platforms, just ignore
            let _ = message;
        }
        
        Ok(())
    }
}

impl ConsoleOutput for WasmBridgeConsoleOutput {
    fn write_text(&self, text: &str) -> ConsoleResult<()> {
        let message = format!("write_text:{}", text);
        self.send_message(&message)
    }
    
    fn write_styled_text(&self, text: &str, style: &TextStyle) -> ConsoleResult<()> {
        // In a full implementation, this would serialize the style
        let message = format!("write_styled_text:{}:{:?}", text, style);
        self.send_message(&message)
    }
    
    fn write_safe_text(&self, text: &str) -> ConsoleResult<()> {
        let message = format!("write_safe_text:{}", text);
        self.send_message(&message)
    }
    
    fn move_cursor_to(&self, row: u16, col: u16) -> ConsoleResult<()> {
        if let Ok(mut pos) = self.cursor_position.lock() {
            *pos = (row, col);
        }
        let message = format!("move_cursor_to:{}:{}", row, col);
        self.send_message(&message)
    }
    
    fn move_cursor_relative(&self, row_delta: i16, col_delta: i16) -> ConsoleResult<()> {
        if let Ok(mut pos) = self.cursor_position.lock() {
            pos.0 = (pos.0 as i16 + row_delta).max(0) as u16;
            pos.1 = (pos.1 as i16 + col_delta).max(0) as u16;
        }
        let message = format!("move_cursor_relative:{}:{}", row_delta, col_delta);
        self.send_message(&message)
    }
    
    fn clear(&self, clear_type: ClearType) -> ConsoleResult<()> {
        let message = format!("clear:{:?}", clear_type);
        self.send_message(&message)
    }
    
    fn set_style(&self, style: &TextStyle) -> ConsoleResult<()> {
        if let Ok(mut current) = self.current_style.lock() {
            *current = style.clone();
        }
        let message = format!("set_style:{:?}", style);
        self.send_message(&message)
    }
    
    fn reset_style(&self) -> ConsoleResult<()> {
        if let Ok(mut current) = self.current_style.lock() {
            *current = TextStyle::default();
        }
        self.send_message("reset_style")
    }
    
    fn flush(&self) -> ConsoleResult<()> {
        self.send_message("flush")
    }
    
    fn set_alternate_screen(&self, enabled: bool) -> ConsoleResult<()> {
        let message = format!("set_alternate_screen:{}", enabled);
        self.send_message(&message)
    }
    
    fn set_cursor_visible(&self, visible: bool) -> ConsoleResult<()> {
        let message = format!("set_cursor_visible:{}", visible);
        self.send_message(&message)
    }
    
    fn get_cursor_position(&self) -> ConsoleResult<(u16, u16)> {
        // Return cached position - in a full implementation, this might query the host
        Ok(*self.cursor_position.lock().unwrap())
    }
    
    fn get_capabilities(&self) -> OutputCapabilities {
        OutputCapabilities {
            supports_colors: true,
            supports_true_color: true,
            supports_styling: true,
            supports_alternate_screen: true,
            supports_cursor_control: true,
            max_colors: 65535,
            platform_name: "WASM Bridge".to_string(),
            backend_type: BackendType::WasmBridge,
        }
    }
}