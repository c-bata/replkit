// Basic integration test to verify test infrastructure
// This test validates that the test infrastructure is properly set up

mod common;

use common::*;

#[test]
fn test_infrastructure_setup() {
    // Test that we can create a test console
    let console = TestConsoleOutput::new(80, 24);
    assert_eq!(console.get_operations().len(), 0);
    
    // Test that terminal emulator works
    let (cols, rows) = console.terminal_emulator().get_size();
    assert_eq!(cols, 80);
    assert_eq!(rows, 24);
    
    // Test cursor position
    let cursor_pos = console.terminal_emulator().get_cursor_position();
    assert_eq!(cursor_pos, (0, 0));
}

#[test]
fn test_config_validation() {
    // Test default configuration
    let config = TestConfig::default();
    assert!(config.validate().is_ok());
    
    // Test invalid configuration
    let invalid_config = TestConfig::new().with_terminal_size(0, 0);
    assert!(invalid_config.validate().is_err());
}

#[test]
fn test_snapshot_manager() {
    let console = TestConsoleOutput::new(80, 24);
    let snapshot_manager = SnapshotManager::new("test_infrastructure");
    
    // This should not panic
    snapshot_manager.assert_terminal_snapshot(&console);
    snapshot_manager.assert_operations_snapshot(&console);
}

#[test]
fn test_compatibility_validator() {
    let validator = CompatibilityValidator::new();
    let console = TestConsoleOutput::new(80, 24);
    
    // Test with non-existent scenario
    let result = validator.validate_scenario("non_existent", &console);
    assert!(!result.matches);
    assert!(!result.differences.is_empty());
}

#[test]
fn test_common_scenarios() {
    let scenarios = CommonScenarios::all();
    assert!(!scenarios.is_empty());
    
    // Verify each scenario has a name and description
    for scenario in scenarios {
        assert!(!scenario.name.is_empty());
        assert!(!scenario.description.is_empty());
        assert!(scenario.config.validate().is_ok());
    }
}