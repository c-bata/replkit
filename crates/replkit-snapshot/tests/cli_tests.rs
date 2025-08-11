use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("replkit-snapshot").unwrap();
    cmd.arg("--help");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Snapshot testing tool"));
}

#[test]
fn test_run_command_help() {
    let mut cmd = Command::cargo_bin("replkit-snapshot").unwrap();
    cmd.args(["run", "--help"]);
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Command to execute"));
}

#[test]
fn test_run_command_missing_required_args() {
    let mut cmd = Command::cargo_bin("replkit-snapshot").unwrap();
    cmd.arg("run");
    
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_run_command_with_all_args() {
    let temp_dir = TempDir::new().unwrap();
    let steps_file = temp_dir.path().join("steps.yaml");
    let compare_dir = temp_dir.path().join("snapshots");
    
    std::fs::write(&steps_file, "").unwrap();
    std::fs::create_dir(&compare_dir).unwrap();
    
    let mut cmd = Command::cargo_bin("replkit-snapshot").unwrap();
    cmd.args([
        "run",
        "--cmd", "echo hello",
        "--steps", steps_file.to_str().unwrap(),
        "--compare", compare_dir.to_str().unwrap(),
        "--winsize", "100x30",
        "--timeout", "5s",
        "--env", "LANG=en_US.UTF-8",
        "--env", "TERM=xterm-256color",
        "--update",
        "--strip-ansi",
        "--idle-wait", "50ms",
    ]);
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Running snapshot test"))
        .stdout(predicate::str::contains("Command: echo hello"))
        .stdout(predicate::str::contains("Terminal size: 100x30"))
        .stdout(predicate::str::contains("Update mode: true"))
        .stdout(predicate::str::contains("LANG=en_US.UTF-8"))
        .stdout(predicate::str::contains("TERM=xterm-256color"));
}

#[test]
fn test_invalid_window_size() {
    let temp_dir = TempDir::new().unwrap();
    let steps_file = temp_dir.path().join("steps.yaml");
    let compare_dir = temp_dir.path().join("snapshots");
    
    std::fs::write(&steps_file, "").unwrap();
    std::fs::create_dir(&compare_dir).unwrap();
    
    let mut cmd = Command::cargo_bin("replkit-snapshot").unwrap();
    cmd.args([
        "run",
        "--cmd", "echo hello",
        "--steps", steps_file.to_str().unwrap(),
        "--compare", compare_dir.to_str().unwrap(),
        "--winsize", "invalid",
    ]);
    
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Invalid window size"));
}

#[test]
fn test_invalid_env_var() {
    let temp_dir = TempDir::new().unwrap();
    let steps_file = temp_dir.path().join("steps.yaml");
    let compare_dir = temp_dir.path().join("snapshots");
    
    std::fs::write(&steps_file, "").unwrap();
    std::fs::create_dir(&compare_dir).unwrap();
    
    let mut cmd = Command::cargo_bin("replkit-snapshot").unwrap();
    cmd.args([
        "run",
        "--cmd", "echo hello",
        "--steps", steps_file.to_str().unwrap(),
        "--compare", compare_dir.to_str().unwrap(),
        "--env", "INVALID_FORMAT",
    ]);
    
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Invalid environment variable"));
}

#[test]
fn test_invalid_duration() {
    let temp_dir = TempDir::new().unwrap();
    let steps_file = temp_dir.path().join("steps.yaml");
    let compare_dir = temp_dir.path().join("snapshots");
    
    std::fs::write(&steps_file, "").unwrap();
    std::fs::create_dir(&compare_dir).unwrap();
    
    let mut cmd = Command::cargo_bin("replkit-snapshot").unwrap();
    cmd.args([
        "run",
        "--cmd", "echo hello",
        "--steps", steps_file.to_str().unwrap(),
        "--compare", compare_dir.to_str().unwrap(),
        "--timeout", "invalid",
    ]);
    
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Invalid duration"));
}