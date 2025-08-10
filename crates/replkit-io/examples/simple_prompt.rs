use replkit::prelude::*;

fn completer(document: &Document) -> Vec<Suggestion> {
    vec![
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
    ]
    .into_iter()
    .filter(|s| {
        document.get_word_before_cursor()
            .chars()
            .zip(s.text.chars())
            .all(|(a, b)| a.to_lowercase().eq(b.to_lowercase()))
    })
    .collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Please select table.");
    
    let mut prompt = Prompt::builder()
        .with_prefix("> ")
        .with_completer(completer)
        .build()?;
    
    let result = prompt.input()?;
    println!("You selected {}", result);
    
    Ok(())
}
