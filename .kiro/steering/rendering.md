# Rendering Strategy Comparison: go-prompt vs replkit

## Overview

This document outlines the fundamental differences in rendering and completion strategies between go-prompt and replkit, explaining why certain design decisions were made.

## Design Philosophies

### go-prompt: "Rendering-Driven" Strategy

go-prompt employs a **non-destructive rendering** approach where completions are applied at display time without modifying the underlying buffer.

**Key Characteristics:**
- **Buffer Immutability**: The text buffer remains unchanged during completion preview
- **Overlay Rendering**: Completions are visually overlaid during rendering phase
- **Relative Positioning**: Uses relative positions within `TextBeforeCursor()` slices
- **Preview-First**: Shows completion effects before committing to buffer changes

**Implementation Pattern:**
```go
// go-prompt approach
cursor = r.backward(cursor, runewidth.StringWidth(
    buffer.Document().GetWordBeforeCursorUntilSeparator(completion.wordSeparator)
))
r.out.WriteStr(suggest.Text)  // Visual overlay only
```

**Flow Diagram:**
```
Input → Buffer (unchanged) → Rendering (+completion overlay) → Display
         ↑ immutable        ↑ composition happens here
```

### replkit: "Buffer-Driven" Strategy

replkit employs a **direct manipulation** approach where completions immediately modify the buffer content.

**Key Characteristics:**
- **Immediate Application**: Tab key directly modifies buffer content
- **Physical Replacement**: Actual string deletion and insertion operations
- **Absolute Positioning**: Uses absolute positions within the entire document
- **State Consistency**: Display state always matches internal buffer state

**Implementation Pattern:**
```rust
// replkit approach
let word_start = doc.find_start_of_word();           // absolute position
let word_len = doc.cursor_position() - word_start;
self.buffer.delete_before_cursor(word_len);          // physical deletion
self.buffer.insert_text(&selected.text, ...);       // physical insertion
```

**Flow Diagram:**
```
Input → Buffer Modification (completion applied) → Rendering → Display
        ↑ immediate change                       ↑ simple display
```

## Technical Implications

### Why replkit Needs `find_start_of_word()`

**go-prompt's `FindStartOfPreviousWord()`:**
- Returns relative position within `TextBeforeCursor()` slice
- Used for rendering calculations and visual positioning
- Sufficient for overlay-based completion display

**replkit's `find_start_of_word()`:**
- Returns absolute position within entire document
- Required for precise buffer manipulation operations
- Enables accurate string deletion before replacement

### API Design Differences

| Aspect | go-prompt | replkit |
|--------|-----------|---------|
| Completion Preview | Visual overlay | Immediate application |
| Buffer State | Unchanged during preview | Modified immediately |
| Position Calculation | Relative to cursor slice | Absolute in document |
| Undo/Redo Complexity | Minimal (no buffer changes) | Requires state tracking |
| Visual Feedback | Preview before commit | Direct modification |

## Rationale for Different Approaches

### go-prompt Advantages
1. **Non-destructive Preview**: Users can see completion effects without commitment
2. **Simplified State Management**: No need for complex undo mechanisms
3. **Rendering Flexibility**: Easy to implement various visual effects
4. **History Preservation**: Buffer history remains clean until final selection

### replkit Advantages
1. **Immediate Feedback**: What you see is exactly what's in the buffer
2. **Rust Ownership Model**: Aligns with Rust's clear ownership semantics
3. **Simplified Rendering**: Display logic is straightforward
4. **State Consistency**: No discrepancy between visual and internal state

## Implementation Context

### Why replkit Chose Buffer-Driven Strategy

1. **Rust Ownership Semantics**: Direct manipulation fits better with Rust's ownership model
2. **Terminal Rendering Complexity**: Avoiding the need to maintain separate display/buffer states
3. **Cross-Platform Consistency**: Simpler to ensure consistent behavior across platforms
4. **Performance Considerations**: Fewer rendering calculations required

### Technical Requirements

The buffer-driven approach necessitated:
- **Absolute Position Tracking**: `find_start_of_word()` for precise manipulation
- **Word Boundary Detection**: Accurate identification of completion targets
- **Cursor Management**: Proper cursor positioning after text replacement

## Conclusion

Both strategies are valid architectural choices with different trade-offs:

- **go-prompt** optimizes for user experience with preview capabilities and non-destructive operations
- **replkit** optimizes for implementation simplicity and state consistency

The choice of buffer-driven strategy in replkit directly led to the need for absolute positioning methods like `find_start_of_word()`, which are unnecessary in go-prompt's overlay-based approach.
