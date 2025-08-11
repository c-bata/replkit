# Replkit

Interactive prompt library for building command-line applications with features like auto-completion, history, and rich text input.

## Features

- **Executor API**: Similar to go-prompt's `Run()` functionality for continuous interactive loops
- **Flexible completion system**: Support for both static completions and dynamic function-based completers
- **Exit checking**: Configurable exit conditions for interactive sessions
- **Unicode support**: Proper handling of international text and emoji
- **Cross-platform**: Works on Windows, macOS, and Linux

## Quick Start

### Basic Input Mode

```rust
use replkit::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut prompt = Prompt::builder()
        .with_prefix(">>> ")
        .build()?;

    let input = prompt.input()?;
    println!("You entered: {}", input);
    Ok(())
}
```

### Executor Mode (go-prompt style)

```rust
use replkit::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut prompt = Prompt::builder()
        .with_prefix("cmd> ")
        .with_completer(StaticCompleter::from_strings(vec![
            "help", "quit", "status"
        ]))
        .with_exit_checker(|input: &str, _breakline: bool| {
            input.trim() == "quit"
        })
        .build()?;

    prompt.run(|input: &str| -> PromptResult<()> {
        match input.trim() {
            "help" => println!("Available commands: help, quit, status"),
            "status" => println!("System is running"),
            "quit" => println!("Goodbye!"),
            _ => println!("Unknown command: {}", input),
        }
        Ok(())
    })?;

    Ok(())
}
```

### With Dynamic Completion

```rust
use replkit::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut prompt = Prompt::builder()
        .with_prefix("sql> ")
        .with_completer(|document: &Document| -> Vec<Suggestion> {
            let word = document.get_word_before_cursor().to_uppercase();
            let mut suggestions = Vec::new();
            
            // SQL keywords
            for keyword in ["SELECT", "INSERT", "UPDATE", "DELETE", "FROM", "WHERE"] {
                if keyword.starts_with(&word) {
                    suggestions.push(Suggestion::new(keyword.to_lowercase(), "SQL keyword"));
                }
            }
            
            suggestions
        })
        .build()?;

    let input = prompt.input()?;
    println!("SQL: {}", input);
    Ok(())
}
```

## API Comparison with go-prompt

### go-prompt
```go
func executor(in string) {
    fmt.Println("You entered:", in)
}

func completer(d prompt.Document) []prompt.Suggest {
    return []prompt.Suggest{
        {Text: "users", Description: "User table"},
        {Text: "articles", Description: "Article table"},
    }
}

func main() {
    p := prompt.New(executor, completer)
    p.Run()
}
```

### replkit
```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut prompt = Prompt::builder()
        .with_completer(|_doc: &Document| -> Vec<Suggestion> {
            vec![
                Suggestion::new("users", "User table"),
                Suggestion::new("articles", "Article table"),
            ]
        })
        .build()?;

    prompt.run(|input: &str| -> PromptResult<()> {
        println!("You entered: {}", input);
        Ok(())
    })?;

    Ok(())
}
```

## Examples

See the `examples/` directory for more complete examples:

- `executor_example.rs` - Basic command prompt with executor
- `sql_prompt.rs` - SQL-like prompt with advanced completion

Run examples with:
```bash
cargo run --example executor_example
cargo run --example sql_prompt
```

## Architecture

Replkit is organized into several layers:

- **Low-level primitives** (`replkit-core`): Document, Buffer, KeyParser, Unicode handling
- **Platform I/O** (`replkit-io`): Cross-platform terminal input/output implementations  
- **High-level API** (`replkit`): Prompt, completion, rendering
