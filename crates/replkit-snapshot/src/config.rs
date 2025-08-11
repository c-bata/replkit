use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use crate::error::{ConfigError, Result};

#[derive(Debug, Deserialize, Serialize)]
pub struct StepDefinition {
    pub version: u32,
    pub command: CommandConfig,
    pub tty: TtyConfig,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommandConfig {
    pub exec: Vec<String>,
    pub workdir: Option<PathBuf>,
    pub env: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TtyConfig {
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Step {
    Send { send: InputSpec },
    WaitIdle { 
        #[serde(rename = "waitIdle")]
        wait_idle: String 
    },
    WaitRegex { 
        #[serde(rename = "waitForRegex")]
        wait_for_regex: String 
    },
    WaitExit { 
        #[serde(rename = "waitExit")]
        wait_exit: String 
    },
    Snapshot { snapshot: SnapshotConfig },
    Sleep { sleep: String },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum InputSpec {
    Text(String),
    Keys(Vec<String>),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SnapshotConfig {
    pub name: String,
    #[serde(default = "default_strip_ansi")]
    #[serde(rename = "stripAnsi")]
    pub strip_ansi: bool,
    pub mask: Option<Vec<String>>,
}

fn default_strip_ansi() -> bool {
    true
}

impl StepDefinition {
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(&path)
            .map_err(|_| ConfigError::StepFileNotFound(path.as_ref().to_path_buf()))?;
        
        // Try YAML first, then JSON
        if let Ok(definition) = serde_yaml::from_str(&content) {
            return Ok(definition);
        }
        
        #[cfg(feature = "json-output")]
        if let Ok(definition) = serde_json::from_str(&content) {
            return Ok(definition);
        }
        
        Err(ConfigError::InvalidStepDefinition(
            "Failed to parse as YAML or JSON".to_string()
        ).into())
    }
    
    pub fn validate(&self) -> Result<()> {
        // Validate version
        if self.version != 1 {
            return Err(ConfigError::InvalidStepDefinition(
                format!("Unsupported version: {}. Only version 1 is supported.", self.version)
            ).into());
        }
        
        // Validate command config
        self.command.validate()?;
        
        // Validate TTY config
        self.tty.validate()?;
        
        // Validate steps
        for (i, step) in self.steps.iter().enumerate() {
            step.validate().map_err(|e| {
                ConfigError::InvalidStepDefinition(
                    format!("Step {}: {}", i, e)
                )
            })?;
        }
        
        Ok(())
    }
}

impl CommandConfig {
    pub fn validate(&self) -> Result<()> {
        if self.exec.is_empty() {
            return Err(ConfigError::InvalidStepDefinition(
                "Command exec array cannot be empty".to_string()
            ).into());
        }
        
        if let Some(workdir) = &self.workdir {
            if !workdir.exists() {
                return Err(ConfigError::InvalidStepDefinition(
                    format!("Working directory does not exist: {}", workdir.display())
                ).into());
            }
        }
        
        Ok(())
    }
}

impl TtyConfig {
    pub fn validate(&self) -> Result<()> {
        if self.cols == 0 || self.rows == 0 {
            return Err(ConfigError::InvalidStepDefinition(
                "TTY dimensions must be greater than 0".to_string()
            ).into());
        }
        
        if self.cols > 1000 || self.rows > 1000 {
            return Err(ConfigError::InvalidStepDefinition(
                "TTY dimensions are unreasonably large".to_string()
            ).into());
        }
        
        Ok(())
    }
}

impl Step {
    pub fn validate(&self) -> Result<()> {
        match self {
            Step::Send { send } => send.validate(),
            Step::WaitIdle { wait_idle } => {
                parse_duration(wait_idle)?;
                Ok(())
            },
            Step::WaitRegex { wait_for_regex } => {
                if wait_for_regex.is_empty() {
                    return Err(ConfigError::InvalidStepDefinition(
                        "waitForRegex cannot be empty".to_string()
                    ).into());
                }
                
                // Validate regex syntax
                regex::Regex::new(wait_for_regex).map_err(|e| {
                    ConfigError::InvalidStepDefinition(
                        format!("Invalid regex in waitForRegex: {}", e)
                    )
                })?;
                
                Ok(())
            },
            Step::WaitExit { wait_exit } => {
                parse_duration(wait_exit)?;
                Ok(())
            },
            Step::Snapshot { snapshot } => snapshot.validate(),
            Step::Sleep { sleep } => {
                parse_duration(sleep)?;
                Ok(())
            },
        }
    }
}

impl InputSpec {
    pub fn validate(&self) -> Result<()> {
        match self {
            InputSpec::Text(text) => {
                if text.is_empty() {
                    return Err(ConfigError::InvalidStepDefinition(
                        "Text input cannot be empty".to_string()
                    ).into());
                }
                Ok(())
            },
            InputSpec::Keys(keys) => {
                if keys.is_empty() {
                    return Err(ConfigError::InvalidStepDefinition(
                        "Key input array cannot be empty".to_string()
                    ).into());
                }
                
                for key in keys {
                    validate_key_spec(key)?;
                }
                
                Ok(())
            },
        }
    }
}

impl SnapshotConfig {
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(ConfigError::InvalidStepDefinition(
                "Snapshot name cannot be empty".to_string()
            ).into());
        }
        
        // Validate snapshot name doesn't contain path separators
        if self.name.contains('/') || self.name.contains('\\') {
            return Err(ConfigError::InvalidStepDefinition(
                "Snapshot name cannot contain path separators".to_string()
            ).into());
        }
        
        Ok(())
    }
}

fn parse_duration(duration_str: &str) -> Result<std::time::Duration> {
    if duration_str.ends_with("ms") {
        let millis = duration_str.trim_end_matches("ms").parse::<u64>()
            .map_err(|_| ConfigError::InvalidDuration(duration_str.to_string()))?;
        Ok(std::time::Duration::from_millis(millis))
    } else if duration_str.ends_with('s') {
        let seconds = duration_str.trim_end_matches('s').parse::<u64>()
            .map_err(|_| ConfigError::InvalidDuration(duration_str.to_string()))?;
        Ok(std::time::Duration::from_secs(seconds))
    } else {
        Err(ConfigError::InvalidDuration(duration_str.to_string()).into())
    }
}

fn validate_key_spec(key: &str) -> Result<()> {
    // List of supported keys
    let supported_keys = [
        "Tab", "Enter", "Esc", "Left", "Right", "Up", "Down",
        "Home", "End", "PageUp", "PageDown", "Delete", "Backspace",
        "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12",
    ];
    
    // Check for simple keys
    if supported_keys.contains(&key) {
        return Ok(());
    }
    
    // Check for modifier combinations
    if key.contains('+') {
        let parts: Vec<&str> = key.split('+').collect();
        if parts.len() == 2 {
            let modifier = parts[0];
            let key_part = parts[1];
            
            let valid_modifiers = ["Ctrl", "Alt", "Shift"];
            if valid_modifiers.contains(&modifier) {
                // Check if the key part is valid (single character or special key)
                if key_part.len() == 1 || supported_keys.contains(&key_part) {
                    return Ok(());
                }
            }
        }
    }
    
    // Check for repeat syntax (e.g., "Left*3")
    if key.contains('*') {
        let parts: Vec<&str> = key.split('*').collect();
        if parts.len() == 2 {
            let key_part = parts[0];
            let repeat_str = parts[1];
            
            if supported_keys.contains(&key_part) && repeat_str.parse::<u32>().is_ok() {
                return Ok(());
            }
        }
    }
    
    Err(ConfigError::InvalidStepDefinition(
        format!("Unsupported key specification: {}", key)
    ).into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;
    
    #[test]
    fn test_parse_valid_yaml() {
        let yaml_content = r#"
version: 1
command:
  exec: ["echo", "hello"]
  workdir: "."
  env:
    LANG: "en_US.UTF-8"
tty:
  cols: 80
  rows: 24
steps:
  - send: "hello world"
  - send: ["Tab"]
  - waitIdle: "100ms"
  - snapshot:
      name: "test-snapshot"
      stripAnsi: true
  - waitExit: "5s"
"#;
        
        let definition: StepDefinition = serde_yaml::from_str(yaml_content).unwrap();
        assert_eq!(definition.version, 1);
        assert_eq!(definition.command.exec, vec!["echo", "hello"]);
        assert_eq!(definition.tty.cols, 80);
        assert_eq!(definition.steps.len(), 5);
    }
    
    #[test]
    fn test_step_definition_validation() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, r#"
version: 1
command:
  exec: ["echo", "hello"]
tty:
  cols: 80
  rows: 24
steps:
  - send: "test"
"#).unwrap();
        
        let definition = StepDefinition::from_file(temp_file.path()).unwrap();
        assert!(definition.validate().is_ok());
    }
    
    #[test]
    fn test_invalid_version() {
        let yaml_content = r#"
version: 2
command:
  exec: ["echo"]
tty:
  cols: 80
  rows: 24
steps: []
"#;
        
        let definition: StepDefinition = serde_yaml::from_str(yaml_content).unwrap();
        assert!(definition.validate().is_err());
    }
    
    #[test]
    fn test_validate_key_specs() {
        assert!(validate_key_spec("Tab").is_ok());
        assert!(validate_key_spec("Ctrl+C").is_ok());
        assert!(validate_key_spec("Alt+F").is_ok());
        assert!(validate_key_spec("Left*3").is_ok());
        assert!(validate_key_spec("InvalidKey").is_err());
        assert!(validate_key_spec("Ctrl+Invalid").is_err());
    }
    
    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("100ms").unwrap(), std::time::Duration::from_millis(100));
        assert_eq!(parse_duration("5s").unwrap(), std::time::Duration::from_secs(5));
        assert!(parse_duration("invalid").is_err());
    }
    
    #[test]
    fn test_tty_config_validation() {
        let valid_tty = TtyConfig { cols: 80, rows: 24 };
        assert!(valid_tty.validate().is_ok());
        
        let invalid_tty = TtyConfig { cols: 0, rows: 24 };
        assert!(invalid_tty.validate().is_err());
        
        let too_large_tty = TtyConfig { cols: 2000, rows: 24 };
        assert!(too_large_tty.validate().is_err());
    }
}