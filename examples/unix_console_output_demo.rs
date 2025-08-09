use prompt_io::create_console_output;
use prompt_core::{TextStyle, Color, ClearType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create console output
    let output = create_console_output()?;
    
    println!("Unix Console Output Demo");
    println!("========================");
    
    // Test basic text output
    output.write_text("Basic text output\n")?;
    
    // Test styled text
    let red_style = TextStyle {
        foreground: Some(Color::Red),
        bold: true,
        ..Default::default()
    };
    output.write_styled_text("Red bold text\n", &red_style)?;
    
    // Test RGB colors
    let rgb_style = TextStyle {
        foreground: Some(Color::Rgb(255, 128, 64)),
        background: Some(Color::Rgb(64, 128, 255)),
        ..Default::default()
    };
    output.write_styled_text("RGB colored text\n", &rgb_style)?;
    
    // Test 256-color
    let ansi256_style = TextStyle {
        foreground: Some(Color::Ansi256(196)), // Bright red
        ..Default::default()
    };
    output.write_styled_text("256-color text\n", &ansi256_style)?;
    
    // Test all text attributes
    let complex_style = TextStyle {
        foreground: Some(Color::Green),
        background: Some(Color::Black),
        bold: true,
        italic: true,
        underline: true,
        ..Default::default()
    };
    output.write_styled_text("Complex styled text\n", &complex_style)?;
    
    // Test cursor movement
    output.write_text("Moving cursor to position (5, 10): ")?;
    output.move_cursor_to(5, 10)?;
    output.write_text("Here!\n")?;
    
    // Test relative cursor movement
    output.write_text("Moving cursor relatively: ")?;
    output.move_cursor_relative(-1, 5)?;
    output.write_text("Moved!\n")?;
    
    // Test clearing
    output.write_text("This line will be cleared...")?;
    std::thread::sleep(std::time::Duration::from_millis(1000));
    output.clear(ClearType::CurrentLine)?;
    output.write_text("Line cleared and replaced!\n")?;
    
    // Test cursor visibility
    output.write_text("Hiding cursor for 2 seconds...")?;
    output.set_cursor_visible(false)?;
    output.flush()?;
    std::thread::sleep(std::time::Duration::from_millis(2000));
    output.set_cursor_visible(true)?;
    output.write_text(" Cursor restored!\n")?;
    
    // Test safe text output
    let unsafe_text = "Safe text: \x1b[31mThis should not be red\x1b[0m";
    output.write_safe_text(unsafe_text)?;
    output.write_text("\n")?;
    
    // Show capabilities
    let caps = output.get_capabilities();
    println!("Platform: {}", caps.platform_name);
    println!("Supports colors: {}", caps.supports_colors);
    println!("Supports true color: {}", caps.supports_true_color);
    println!("Supports styling: {}", caps.supports_styling);
    println!("Max colors: {}", caps.max_colors);
    
    output.write_text("\nDemo completed!\n")?;
    output.flush()?;
    
    Ok(())
}