//! SQL-like prompt example demonstrating advanced Executor usage
//!
//! This example mimics a simple SQL prompt with table completion,
//! similar to the go-prompt SQL example.
//!
//! Run with: cargo run --example sql_prompt

use replkit::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Simple SQL Prompt");
    println!("Available tables: users, articles, comments");
    println!("Commands: SELECT, INSERT, UPDATE, DELETE, SHOW TABLES, EXIT");
    println!();

    // Create a completer for SQL-like commands
    let completer = |document: &Document| -> Vec<Suggestion> {
        let text = document.text().to_uppercase();
        let word = document.get_word_before_cursor().to_uppercase();

        let mut suggestions = Vec::new();

        // SQL keywords
        let keywords = vec![
            ("SELECT", "Select data from tables"),
            ("INSERT", "Insert data into tables"),
            ("UPDATE", "Update existing data"),
            ("DELETE", "Delete data from tables"),
            ("FROM", "Specify source table"),
            ("WHERE", "Add conditions"),
            ("ORDER BY", "Sort results"),
            ("GROUP BY", "Group results"),
            ("SHOW TABLES", "List all tables"),
            ("EXIT", "Exit the SQL prompt"),
        ];

        // Table names
        let tables = vec![
            ("users", "Store username and age"),
            ("articles", "Store article text posted by users"),
            ("comments", "Store comments on articles"),
        ];

        // Add keyword suggestions
        for (keyword, description) in keywords {
            if keyword.starts_with(&word) {
                suggestions.push(Suggestion::new(keyword.to_lowercase(), description));
            }
        }

        // Add table suggestions if we're after FROM or similar
        if text.contains("FROM") || text.contains("UPDATE") || text.contains("INSERT INTO") {
            for (table, description) in tables {
                if table.to_uppercase().starts_with(&word) {
                    suggestions.push(Suggestion::new(table, description));
                }
            }
        }

        suggestions
    };

    let mut prompt = Prompt::builder()
        .with_prefix("sql> ")
        .with_completer(completer)
        .with_exit_checker(|input: &str, _breakline: bool| input.trim().to_uppercase() == "EXIT")
        .build()?;

    let result = prompt.run(|input: &str| -> PromptResult<()> {
        let input = input.trim();

        if input.is_empty() {
            return Ok(());
        }

        let upper_input = input.to_uppercase();

        match upper_input.as_str() {
            "SHOW TABLES" => {
                println!("Tables:");
                println!("  users     - Store username and age");
                println!("  articles  - Store article text posted by users");
                println!("  comments  - Store comments on articles");
            }
            "EXIT" => {
                println!("Goodbye!");
            }
            _ => {
                if upper_input.starts_with("SELECT") {
                    println!("Executing query: {}", input);
                    println!("(This is a mock SQL prompt - no actual database)");
                } else if upper_input.starts_with("INSERT") {
                    println!("Inserting data: {}", input);
                    println!("(This is a mock SQL prompt - no actual database)");
                } else if upper_input.starts_with("UPDATE") {
                    println!("Updating data: {}", input);
                    println!("(This is a mock SQL prompt - no actual database)");
                } else if upper_input.starts_with("DELETE") {
                    println!("Deleting data: {}", input);
                    println!("(This is a mock SQL prompt - no actual database)");
                } else {
                    println!("Unknown SQL command: {}", input);
                    println!("Try: SELECT, INSERT, UPDATE, DELETE, SHOW TABLES, or EXIT");
                }
            }
        }

        Ok(())
    });

    match result {
        Ok(()) => println!("\nSQL session ended"),
        Err(PromptError::Interrupted) => println!("\nSQL session interrupted"),
        Err(e) => eprintln!("Error: {}", e),
    }

    Ok(())
}
