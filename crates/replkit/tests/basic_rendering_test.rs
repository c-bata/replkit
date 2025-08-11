// Basic rendering test to demonstrate the test infrastructure
// This test shows how to use the testing framework for rendering validation

mod common;

use common::*;

#[test]
fn test_terminal_emulator_basic_operations() {
    let mut emulator = TerminalEmulator::new(80, 24);
    
    // Test initial state
    assert_eq!(emulator.get_cursor_position(), (0, 0));
    assert_eq!(emulator.get_size(), (80, 24));
    
    // Test cursor movement
    emulator.move_cursor_to(5, 10).unwrap();
    assert_eq!(emulator.get_cursor_position(), (5, 10));
    
    // Test writing text
    emulator.write_text("Hello").unwrap();
    let screen_contents = emulator.get_screen_contents();
    assert!(screen_contents.contains("Hello"));
}

#[test]
fn test_console_output_capture() {
    let console = TestConsoleOutput::new(80, 24);
    
    // Test initial state
    assert_eq!(console.get_operations().len(), 0);
    assert_eq!(console.terminal_emulator().get_cursor_position(), (0, 0));
    
    // Test that we can access terminal emulator
    let (cols, rows) = console.terminal_emulator().get_size();
    assert_eq!(cols, 80);
    assert_eq!(rows, 24);
}

#[test]
fn test_snapshot_manager_basic() {
    let console = TestConsoleOutput::new(80, 24);
    let snapshot_manager = SnapshotManager::new("basic_test");
    
    // This should not panic and should print output
    snapshot_manager.assert_terminal_snapshot(&console);
    snapshot_manager.assert_operations_snapshot(&console);
}

#[test]
fn test_compatibility_validator_basic() {
    let validator = CompatibilityValidator::load_references();
    let console = TestConsoleOutput::new(80, 24);
    
    // Test with the built-in basic_prompt scenario
    let result = validator.validate_scenario("basic_prompt", &console);
    
    // Should not match since we haven't set up the expected output
    assert!(!result.matches);
    assert!(!result.differences.is_empty());
}

#[test]
fn test_test_config_builder_pattern() {
    let config = TestConfig::new()
        .with_terminal_size(120, 30)
        .with_timeout(10000);
    
    assert_eq!(config.terminal_size, (120, 30));
    assert_eq!(config.test_timeout_ms, 10000);
    assert!(config.validate().is_ok());
}

#[test]
fn test_common_scenarios_creation() {
    let basic = CommonScenarios::basic_prompt();
    assert_eq!(basic.name, "basic_prompt");
    assert!(!basic.description.is_empty());
    
    let completion = CommonScenarios::completion_menu();
    assert_eq!(completion.name, "completion_menu");
    assert!(!completion.description.is_empty());
    
    let all_scenarios = CommonScenarios::all();
    assert!(all_scenarios.len() >= 4);
}