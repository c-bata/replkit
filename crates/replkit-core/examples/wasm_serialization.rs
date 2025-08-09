//! Example demonstrating WASM serialization for Buffer and Document
//!
//! This example shows how to serialize and deserialize Buffer and Document
//! instances for use with WASM runtimes like wazero.

#[cfg(feature = "wasm")]
fn main() {
    use replkit_core::{Buffer, Document, Key, WasmBufferState, WasmDocumentState};
    use serde_json;

    println!("WASM Serialization Example");
    println!("==========================");

    // Create a Buffer with some content
    let mut buffer = Buffer::new();
    buffer.set_text("Hello 🌍 World!\nSecond line with 测试".to_string());
    buffer.add_working_line("Third line".to_string(), false);
    buffer.set_cursor_position(8); // After "🌍 "
    buffer.set_last_key_stroke(Key::Right);

    println!("\nOriginal Buffer:");
    println!("  Text: {:?}", buffer.text());
    println!("  Cursor position: {}", buffer.cursor_position());
    println!("  Working lines count: {}", buffer.working_lines_count());
    println!("  Working index: {}", buffer.working_index());

    // Serialize Buffer to WASM state
    let buffer_wasm_state = buffer.to_wasm_state();
    println!("\nSerialized Buffer state:");
    println!("  Working lines: {:?}", buffer_wasm_state.working_lines);
    println!("  Cursor position: {}", buffer_wasm_state.cursor_position);
    println!("  Working index: {}", buffer_wasm_state.working_index);

    // Convert to JSON (for wazero runtime)
    let buffer_json = serde_json::to_string_pretty(&buffer_wasm_state)
        .expect("Failed to serialize buffer to JSON");
    println!("\nBuffer as JSON:");
    println!("{}", buffer_json);

    // Deserialize back from JSON
    let buffer_state_from_json: WasmBufferState =
        serde_json::from_str(&buffer_json).expect("Failed to deserialize buffer from JSON");
    let restored_buffer = Buffer::from_wasm_state(buffer_state_from_json);

    println!("\nRestored Buffer:");
    println!("  Text: {:?}", restored_buffer.text());
    println!("  Cursor position: {}", restored_buffer.cursor_position());
    println!(
        "  Working lines count: {}",
        restored_buffer.working_lines_count()
    );
    println!("  Working index: {}", restored_buffer.working_index());

    // Verify roundtrip integrity
    assert_eq!(buffer.text(), restored_buffer.text());
    assert_eq!(buffer.cursor_position(), restored_buffer.cursor_position());
    assert_eq!(
        buffer.working_lines_count(),
        restored_buffer.working_lines_count()
    );
    assert_eq!(buffer.working_index(), restored_buffer.working_index());
    println!("✓ Buffer roundtrip successful!");

    // Document example
    println!("\n{}", "=".repeat(50));

    let document = Document::with_text_and_key(
        "Document with 🚀 emoji and 中文".to_string(),
        15, // After "🚀 "
        Some(Key::F1),
    );

    println!("\nOriginal Document:");
    println!("  Text: {:?}", document.text());
    println!("  Cursor position: {}", document.cursor_position());
    println!(
        "  Display cursor position: {}",
        document.display_cursor_position()
    );
    println!("  Last key: {:?}", document.last_key_stroke());

    // Serialize Document to WASM state
    let doc_wasm_state = document.to_wasm_state();
    let doc_json = serde_json::to_string_pretty(&doc_wasm_state)
        .expect("Failed to serialize document to JSON");

    println!("\nDocument as JSON:");
    println!("{}", doc_json);

    // Deserialize back
    let doc_state_from_json: WasmDocumentState =
        serde_json::from_str(&doc_json).expect("Failed to deserialize document from JSON");
    let restored_document = Document::from_wasm_state(doc_state_from_json);

    println!("\nRestored Document:");
    println!("  Text: {:?}", restored_document.text());
    println!("  Cursor position: {}", restored_document.cursor_position());
    println!(
        "  Display cursor position: {}",
        restored_document.display_cursor_position()
    );
    println!("  Last key: {:?}", restored_document.last_key_stroke());

    // Verify roundtrip integrity
    assert_eq!(document.text(), restored_document.text());
    assert_eq!(
        document.cursor_position(),
        restored_document.cursor_position()
    );
    assert_eq!(
        document.last_key_stroke(),
        restored_document.last_key_stroke()
    );
    println!("✓ Document roundtrip successful!");

    println!("\n🎉 All WASM serialization tests passed!");
}

#[cfg(not(feature = "wasm"))]
fn main() {
    println!("This example requires the 'wasm' feature to be enabled.");
    println!("Run with: cargo run --example wasm_serialization --features wasm");
}
