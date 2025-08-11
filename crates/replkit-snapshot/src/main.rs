use clap::Parser;
use replkit_snapshot::{
    Cli, Commands, RunConfig, StepDefinition, PtyManager, StepExecutor, 
    SnapshotComparator, ComparisonAction, Result
};
use std::process;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();
    
    match &cli.command {
        Commands::Run { .. } => {
            let config = RunConfig::from_cli_args(&cli.command)?;
            run_snapshot_test(config).await
        }
    }
}

async fn run_snapshot_test(config: RunConfig) -> Result<()> {
    println!("Running snapshot test with config:");
    println!("  Command: {}", config.command);
    println!("  Terminal size: {}x{}", config.terminal_size.0, config.terminal_size.1);
    println!("  Step file: {}", config.step_file.display());
    println!("  Snapshot directory: {}", config.snapshot_directory.display());
    println!("  Update mode: {}", config.update_snapshots);
    
    if let Some(workdir) = &config.working_directory {
        println!("  Working directory: {}", workdir.display());
    }
    
    if !config.environment.is_empty() {
        println!("  Environment variables:");
        for (key, value) in &config.environment {
            println!("    {}={}", key, value);
        }
    }
    
    // Load and validate step definition
    println!("\nLoading step definition from: {}", config.step_file.display());
    let step_definition = StepDefinition::from_file(&config.step_file)?;
    step_definition.validate()?;
    
    println!("  Step definition version: {}", step_definition.version);
    println!("  Command: {:?}", step_definition.command.exec);
    println!("  TTY size: {}x{}", step_definition.tty.cols, step_definition.tty.rows);
    println!("  Number of steps: {}", step_definition.steps.len());
    
    // Display steps summary
    if !step_definition.steps.is_empty() {
        println!("\nSteps summary:");
        for (i, step) in step_definition.steps.iter().enumerate() {
            match step {
                replkit_snapshot::Step::Send { send } => {
                    match send {
                        replkit_snapshot::InputSpec::Text(text) => {
                            let display_text = if text.len() > 20 {
                                format!("{}...", &text[..20])
                            } else {
                                text.clone()
                            };
                            println!("  {}: Send text: \"{}\"", i + 1, display_text);
                        },
                        replkit_snapshot::InputSpec::Keys(keys) => {
                            println!("  {}: Send keys: {:?}", i + 1, keys);
                        },
                    }
                },
                replkit_snapshot::Step::WaitIdle { wait_idle } => {
                    println!("  {}: Wait idle: {}", i + 1, wait_idle);
                },
                replkit_snapshot::Step::WaitRegex { wait_for_regex } => {
                    println!("  {}: Wait for regex: \"{}\"", i + 1, wait_for_regex);
                },
                replkit_snapshot::Step::WaitExit { wait_exit } => {
                    println!("  {}: Wait for exit: {}", i + 1, wait_exit);
                },
                replkit_snapshot::Step::Snapshot { snapshot } => {
                    println!("  {}: Take snapshot: \"{}\"", i + 1, snapshot.name);
                },
                replkit_snapshot::Step::Sleep { sleep } => {
                    println!("  {}: Sleep: {}", i + 1, sleep);
                },
            }
        }
    }
    
    // Initialize PTY manager
    println!("\nInitializing PTY with size {}x{}", step_definition.tty.cols, step_definition.tty.rows);
    let mut pty_manager = PtyManager::new(step_definition.tty.cols, step_definition.tty.rows)?;
    
    // Spawn the command from step definition (overrides CLI command)
    println!("Spawning command: {:?}", step_definition.command.exec);
    pty_manager.spawn_command(&step_definition.command)?;
    
    // Wait a moment for the command to start
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    
    // Check if process is running
    if pty_manager.is_process_running() {
        println!("Process is running successfully");
    } else {
        println!("Process has already completed");
    }
    
    // Initialize step executor with screen capturer
    let terminal_size = (step_definition.tty.cols, step_definition.tty.rows);
    let mut step_executor = StepExecutor::new(pty_manager, terminal_size)?;
    
    // Execute all steps
    println!("\nExecuting {} steps...", step_definition.steps.len());
    let execution_results = step_executor.execute_steps(&step_definition.steps).await?;
    
    // Display execution results
    println!("\n=== Execution Summary ===");
    let successful_steps = execution_results.iter().filter(|r| r.success).count();
    let failed_steps = execution_results.len() - successful_steps;
    
    println!("Total steps: {}", execution_results.len());
    println!("Successful: {}", successful_steps);
    println!("Failed: {}", failed_steps);
    
    if !execution_results.is_empty() {
        println!("\nStep details:");
        for result in &execution_results {
            let status = if result.success { "✓" } else { "✗" };
            println!("  {} Step {}: {} ({:?})", 
                status, 
                result.step_index + 1, 
                result.step_name, 
                result.duration
            );
            
            if let Some(error) = &result.error {
                println!("    Error: {}", error);
            }
            
            if let Some(output) = &result.output {
                if !output.is_empty() {
                    let preview = String::from_utf8_lossy(output);
                    let preview = if preview.len() > 100 {
                        format!("{}...", &preview[..100])
                    } else {
                        preview.to_string()
                    };
                    println!("    Raw output: {:?}", preview);
                }
            }
            
            if let Some(snapshot) = &result.snapshot {
                println!("    Snapshot: {} chars, {}x{}", 
                    snapshot.content.len(),
                    snapshot.terminal_size.0,
                    snapshot.terminal_size.1
                );
                if !snapshot.content.trim().is_empty() {
                    let lines: Vec<&str> = snapshot.content.lines().collect();
                    println!("    Screen content ({} lines):", lines.len());
                    for (i, line) in lines.iter().take(3).enumerate() {
                        println!("      [{}] {:?}", i + 1, line);
                    }
                    if lines.len() > 3 {
                        println!("      ... ({} more lines)", lines.len() - 3);
                    }
                }
            }
        }
    }
    
    // Collect snapshots from execution results
    let snapshots: Vec<_> = execution_results
        .iter()
        .filter_map(|result| result.snapshot.as_ref())
        .cloned()
        .collect();
    
    if !snapshots.is_empty() {
        println!("\n=== Snapshot Comparison ===");
        
        // Initialize snapshot comparator
        let comparator = SnapshotComparator::new(
            config.snapshot_directory.clone(),
            config.update_snapshots
        )?;
        
        println!("Comparing {} snapshots against golden files...", snapshots.len());
        println!("Snapshot directory: {}", config.snapshot_directory.display());
        println!("Update mode: {}", config.update_snapshots);
        
        // Compare all snapshots
        let comparison_results = comparator.compare_multiple_snapshots(&snapshots)?;
        
        // Display comparison results
        let mut all_passed = true;
        for result in &comparison_results {
            let status = if result.matches { "✓" } else { "✗" };
            println!("  {} Snapshot '{}': {:?}", 
                status, 
                result.snapshot_name, 
                result.action_taken
            );
            
            if !result.matches {
                all_passed = false;
                if let Some(diff) = &result.diff {
                    println!("    {}", diff.replace('\n', "\n    "));
                }
            } else if let Some(diff) = &result.diff {
                // Show diff for updated files
                match &result.action_taken {
                    ComparisonAction::Updated => {
                        println!("    Updated with changes:");
                        println!("    {}", diff.replace('\n', "\n    "));
                    },
                    ComparisonAction::Created => {
                        println!("    Created new golden file: {}", result.golden_file_path.display());
                    },
                    _ => {}
                }
            }
        }
        
        // Summary
        let passed_count = comparison_results.iter().filter(|r| r.matches).count();
        let failed_count = comparison_results.len() - passed_count;
        
        println!("\n=== Comparison Summary ===");
        println!("Snapshots passed: {}", passed_count);
        println!("Snapshots failed: {}", failed_count);
        
        if !all_passed {
            println!("\nSome snapshots failed comparison!");
            println!("Run with --update to update golden files.");
            std::process::exit(1);
        } else {
            println!("\nAll snapshots match their golden files! ✓");
        }
    } else {
        println!("\n=== No Snapshots to Compare ===");
        println!("No snapshot steps were executed.");
    }
    
    Ok(())
}