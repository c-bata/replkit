use std::time::SystemTime;
use crate::config::SnapshotConfig;
use crate::error::{CaptureError, Result};

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub name: String,
    pub content: String,
    pub raw_content: Option<Vec<u8>>,
    pub timestamp: SystemTime,
    pub terminal_size: (u16, u16),
}

pub struct ScreenCapturer {
    terminal_size: (u16, u16),
    strip_ansi_default: bool,
    accumulated_output: Vec<u8>,
}

impl ScreenCapturer {
    pub fn new(cols: u16, rows: u16, strip_ansi: bool) -> Result<Self> {
        Ok(Self {
            terminal_size: (cols, rows),
            strip_ansi_default: strip_ansi,
            accumulated_output: Vec::new(),
        })
    }
    
    pub fn feed_data(&mut self, data: &[u8]) -> Result<()> {
        // Accumulate all data we receive
        self.accumulated_output.extend_from_slice(data);
        Ok(())
    }
    
    pub fn capture_snapshot(&mut self, config: &SnapshotConfig, raw_data: Option<Vec<u8>>) -> Result<Snapshot> {
        // Get all accumulated output
        let content = if let Some(raw) = &raw_data {
            // Use the provided raw data if available
            String::from_utf8_lossy(raw).to_string()
        } else if !self.accumulated_output.is_empty() {
            // Use accumulated output
            String::from_utf8_lossy(&self.accumulated_output).to_string()
        } else {
            // No data available
            String::new()
        };
        
        // Apply normalization
        let normalized_content = if config.strip_ansi {
            self.strip_ansi_codes(&content)
        } else {
            content
        };
        
        // Apply masks if specified
        let final_content = if let Some(masks) = &config.mask {
            self.apply_mask(&normalized_content, masks)
        } else {
            normalized_content
        };
        
        let final_content = self.normalize_content(&final_content);
        
        Ok(Snapshot {
            name: config.name.clone(),
            content: final_content,
            raw_content: raw_data,
            timestamp: SystemTime::now(),
            terminal_size: self.terminal_size,
        })
    }
    
    pub fn get_screen_lines(&mut self) -> Result<Vec<String>> {
        let content = String::from_utf8_lossy(&self.accumulated_output);
        let lines: Vec<String> = content.lines().map(|line| line.to_string()).collect();
        Ok(lines)
    }
    
    pub fn clear(&mut self) -> Result<()> {
        self.accumulated_output.clear();
        Ok(())
    }
    
    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        self.terminal_size = (cols, rows);
        Ok(())
    }
    
    fn strip_ansi_codes(&self, text: &str) -> String {
        // Simple ANSI stripping using regex
        let re = regex::Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap();
        re.replace_all(text, "").to_string()
    }
    
    fn normalize_content(&self, content: &str) -> String {
        // Normalize line endings and trim trailing spaces
        content.lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
    }
    
    fn apply_mask(&self, content: &str, masks: &[String]) -> String {
        let mut result = content.to_string();
        
        for mask in masks {
            if let Ok(regex) = regex::Regex::new(mask) {
                result = regex.replace_all(&result, "[MASKED]").to_string();
            }
        }
        
        result
    }
    
    pub fn get_terminal_size(&self) -> (u16, u16) {
        self.terminal_size
    }
    
    pub fn get_accumulated_output(&self) -> &[u8] {
        &self.accumulated_output
    }
}

// Content normalization utilities
pub struct ContentNormalizer;

impl ContentNormalizer {
    pub fn normalize_content(content: &str, options: &NormalizationOptions) -> String {
        let mut result = content.to_string();
        
        if options.strip_ansi {
            result = Self::strip_ansi_codes(&result);
        }
        
        if options.trim_trailing_spaces {
            result = Self::trim_trailing_spaces(&result);
        }
        
        if options.normalize_line_endings {
            result = Self::normalize_line_endings(&result);
        }
        
        if let Some(masks) = &options.masks {
            result = Self::apply_masks(&result, masks);
        }
        
        result
    }
    
    fn strip_ansi_codes(text: &str) -> String {
        let re = regex::Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap();
        re.replace_all(text, "").to_string()
    }
    
    fn trim_trailing_spaces(text: &str) -> String {
        text.lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
    }
    
    fn normalize_line_endings(text: &str) -> String {
        text.replace("\r\n", "\n").replace('\r', "\n")
    }
    
    fn apply_masks(text: &str, masks: &[String]) -> String {
        let mut result = text.to_string();
        
        for mask in masks {
            if let Ok(regex) = regex::Regex::new(mask) {
                result = regex.replace_all(&result, "[MASKED]").to_string();
            }
        }
        
        result
    }
}

#[derive(Debug, Clone)]
pub struct NormalizationOptions {
    pub strip_ansi: bool,
    pub trim_trailing_spaces: bool,
    pub normalize_line_endings: bool,
    pub masks: Option<Vec<String>>,
}

impl Default for NormalizationOptions {
    fn default() -> Self {
        Self {
            strip_ansi: true,
            trim_trailing_spaces: true,
            normalize_line_endings: true,
            masks: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_screen_capturer_creation() {
        let capturer = ScreenCapturer::new(80, 24, true);
        assert!(capturer.is_ok());
        
        let capturer = capturer.unwrap();
        assert_eq!(capturer.get_terminal_size(), (80, 24));
    }
    
    #[test]
    fn test_ansi_stripping() {
        let capturer = ScreenCapturer::new(80, 24, true).unwrap();
        let text = "Hello \x1b[31mworld\x1b[0m!";
        let result = capturer.strip_ansi_codes(text);
        assert_eq!(result, "Hello world!");
    }
    
    #[test]
    fn test_content_normalization() {
        let options = NormalizationOptions {
            strip_ansi: true,
            trim_trailing_spaces: true,
            normalize_line_endings: true,
            masks: Some(vec![r"\d{4}-\d{2}-\d{2}".to_string()]), // Date pattern
        };
        
        let content = "Date: 2023-12-25\x1b[31m  \r\nHello world   ";
        let result = ContentNormalizer::normalize_content(content, &options);
        assert_eq!(result, "Date: [MASKED]\nHello world");
    }
    
    #[test]
    fn test_feed_data_and_capture() {
        let mut capturer = ScreenCapturer::new(10, 3, true).unwrap();
        
        // Feed some test data
        capturer.feed_data(b"Hello\nWorld").unwrap();
        
        let config = SnapshotConfig {
            name: "test".to_string(),
            strip_ansi: true,
            mask: None,
        };
        
        let snapshot = capturer.capture_snapshot(&config, None).unwrap();
        assert_eq!(snapshot.name, "test");
        assert_eq!(snapshot.terminal_size, (10, 3));
        assert!(snapshot.content.contains("Hello"));
        assert!(snapshot.content.contains("World"));
    }
    
    #[test]
    fn test_data_accumulation() {
        let mut capturer = ScreenCapturer::new(80, 24, true).unwrap();
        
        // Feed data in chunks
        capturer.feed_data(b"Part 1\n").unwrap();
        capturer.feed_data(b"Part 2\n").unwrap();
        
        let config = SnapshotConfig {
            name: "accumulated".to_string(),
            strip_ansi: true,
            mask: None,
        };
        
        let snapshot = capturer.capture_snapshot(&config, None).unwrap();
        assert!(snapshot.content.contains("Part 1"));
        assert!(snapshot.content.contains("Part 2"));
    }
}