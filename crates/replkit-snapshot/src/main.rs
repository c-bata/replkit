use clap::Parser;
use replkit_snapshot::{Cli, Commands, RunConfig, StepDefinition, PtyManager, Result};
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
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    
    // Check if process is running
    if pty_manager.is_process_running() {
        println!("Process is running successfully");
        
        // Drain initial output
        let initial_output = pty_manager.drain_output(std::time::Duration::from_millis(1000)).await?;
        if !initial_output.is_empty() {
            println!("Initial output received ({} bytes):", initial_output.len());
            let output_str = String::from_utf8_lossy(&initial_output);
            println!("Output preview: {:?}", &output_str[..output_str.len().min(100)]);
        }
    } else {
        println!("Process has already completed");
    }
    
    // TODO: Implement step execution and snapshot capture
    println!("\n[TODO] Step execution and snapshot capture not yet implemented");
    println!("PTY management is now functional!");
    
    Ok(())
}