use replkit::prelude::*;

fn completer(document: &Document) -> Vec<Suggestion> {
    let suggestions = vec![
        Suggestion {
            text: "users".to_string(),
            description: "Store the username and age".to_string(),
        },
        Suggestion {
            text: "articles".to_string(),
            description: "Store the article text posted by user".to_string(),
        },
        Suggestion {
            text: "comments".to_string(),
            description: "Store the text commented to articles".to_string(),
        },
        Suggestion {
            text: "groups".to_string(),
            description: "Combine users with specific rules".to_string(),
        },
    ];
    
    let word_before_cursor = document.get_word_before_cursor();
    suggestions
        .into_iter()
        .filter(|s| {
            s.text.to_lowercase().starts_with(&word_before_cursor.to_lowercase())
        })
        .collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Please select table.");

    let mut prompt = Prompt::builder()
        .with_prefix(">>> ")
        .with_completer(completer)
        .build()?;

    let result = prompt.input()?;
    println!("Your input: {}", result);

    Ok(())
}
