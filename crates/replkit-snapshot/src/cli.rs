use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use crate::error::{ConfigError, Result};

#[derive(Parser)]
#[command(name = "replkit-snapshot")]
#[command(about = "Snapshot testing tool for terminal applications built with replkit")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Run {
        /// Command to execute
        #[arg(long)]
        cmd: String,
        
        /// Working directory
        #[arg(long)]
        workdir: Option<PathBuf>,
        
        /// Environment variables (KEY=VALUE)
        #[arg(long = "env")]
        env_vars: Vec<String>,
        
        /// Terminal window size (COLSxROWS)
        #[arg(long, default_value = "80x24")]
        winsize: String,
        
        /// Step definition file
        #[arg(long)]
        steps: PathBuf,
        
        /// Snapshot directory
        #[arg(long)]
        compare: PathBuf,
        
        /// Update golden snapshots
        #[arg(long)]
        update: bool,
        
        /// Global timeout
        #[arg(long, default_value = "30s")]
        timeout: String,
        
        /// Strip ANSI codes
        #[arg(long, default_value = "true")]
        strip_ansi: bool,
        
        /// Idle wait duration after input
        #[arg(long, default_value = "100ms")]
        idle_wait: String,
    },
}

#[derive(Debug, Clone)]
pub struct RunConfig {
    pub command: String,
    pub working_directory: Option<PathBuf>,
    pub environment: HashMap<String, String>,
    pub terminal_size: (u16, u16),
    pub step_file: PathBuf,
    pub snapshot_directory: PathBuf,
    pub timeout: Duration,
    pub update_snapshots: bool,
    pub strip_ansi: bool,
    pub idle_wait: Duration,
}

impl RunConfig {
    pub fn from_cli_args(args: &Commands) -> Result<Self> {
        match args {
            Commands::Run {
                cmd,
                workdir,
                env_vars,
                winsize,
                steps,
                compare,
                update,
                timeout,
                strip_ansi,
                idle_wait,
            } => {
                let (cols, rows) = Self::parse_window_size(winsize)?;
                let environment = Self::parse_env_vars(env_vars)?;
                let timeout_duration = Self::parse_duration(timeout)?;
                let idle_wait_duration = Self::parse_duration(idle_wait)?;
                
                Ok(Self {
                    command: cmd.clone(),
                    working_directory: workdir.clone(),
                    environment,
                    terminal_size: (cols, rows),
                    step_file: steps.clone(),
                    snapshot_directory: compare.clone(),
                    timeout: timeout_duration,
                    update_snapshots: *update,
                    strip_ansi: *strip_ansi,
                    idle_wait: idle_wait_duration,
                })
            }
        }
    }
    
    fn parse_window_size(winsize: &str) -> Result<(u16, u16)> {
        let parts: Vec<&str> = winsize.split('x').collect();
        if parts.len() != 2 {
            return Err(ConfigError::InvalidWindowSize(winsize.to_string()).into());
        }
        
        let cols = parts[0].parse::<u16>()
            .map_err(|_| ConfigError::InvalidWindowSize(winsize.to_string()))?;
        let rows = parts[1].parse::<u16>()
            .map_err(|_| ConfigError::InvalidWindowSize(winsize.to_string()))?;
        
        Ok((cols, rows))
    }
    
    fn parse_env_vars(env_vars: &[String]) -> Result<HashMap<String, String>> {
        let mut env = HashMap::new();
        
        for var in env_vars {
            let parts: Vec<&str> = var.splitn(2, '=').collect();
            if parts.len() != 2 {
                return Err(ConfigError::InvalidEnvironmentVariable(var.clone()).into());
            }
            env.insert(parts[0].to_string(), parts[1].to_string());
        }
        
        Ok(env)
    }
    
    fn parse_duration(duration_str: &str) -> Result<Duration> {
        if duration_str.ends_with("ms") {
            let millis = duration_str.trim_end_matches("ms").parse::<u64>()
                .map_err(|_| ConfigError::InvalidDuration(duration_str.to_string()))?;
            Ok(Duration::from_millis(millis))
        } else if duration_str.ends_with('s') {
            let seconds = duration_str.trim_end_matches('s').parse::<u64>()
                .map_err(|_| ConfigError::InvalidDuration(duration_str.to_string()))?;
            Ok(Duration::from_secs(seconds))
        } else {
            Err(ConfigError::InvalidDuration(duration_str.to_string()).into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_window_size() {
        assert_eq!(RunConfig::parse_window_size("80x24").unwrap(), (80, 24));
        assert_eq!(RunConfig::parse_window_size("100x30").unwrap(), (100, 30));
        assert!(RunConfig::parse_window_size("invalid").is_err());
        assert!(RunConfig::parse_window_size("80x").is_err());
    }
    
    #[test]
    fn test_parse_env_vars() {
        let vars = vec!["LANG=en_US.UTF-8".to_string(), "TERM=xterm-256color".to_string()];
        let result = RunConfig::parse_env_vars(&vars).unwrap();
        
        assert_eq!(result.get("LANG"), Some(&"en_US.UTF-8".to_string()));
        assert_eq!(result.get("TERM"), Some(&"xterm-256color".to_string()));
        
        let invalid_vars = vec!["INVALID".to_string()];
        assert!(RunConfig::parse_env_vars(&invalid_vars).is_err());
    }
    
    #[test]
    fn test_parse_duration() {
        assert_eq!(RunConfig::parse_duration("30s").unwrap(), Duration::from_secs(30));
        assert_eq!(RunConfig::parse_duration("500ms").unwrap(), Duration::from_millis(500));
        assert!(RunConfig::parse_duration("invalid").is_err());
    }
}