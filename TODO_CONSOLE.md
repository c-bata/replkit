# Console Input API Design and Implementation Plan

## Overview

This document outlines the cross-platform console input API design strategy for replkit, focusing on compatibility across native platforms (Unix/Windows), WASM environments, and Go bindings.

## Design Philosophy

### Core Principles

1. **Cross-Platform Compatibility**: Support Unix/Linux, Windows, WASM, and Go bindings with a unified API
2. **Non-Blocking First**: Prioritize non-blocking operations due to WASM constraints
3. **Clear Intent**: Separate APIs for different use cases to improve code readability
4. **Performance Optimization**: Platform-specific optimizations while maintaining API consistency

### API Design Strategy

#### Synchronous Reading Methods

We've designed two complementary methods for key input:

1. **`try_read_key()`**: Pure non-blocking, immediate return
   - Clear intent: "check if input is available"
   - Optimized implementation path
   - Go-friendly (channel select patterns)

2. **`read_key_timeout(timeout_ms: Option<u32>)`**: Flexible timeout control
   - `Some(0)`: Non-blocking (equivalent to `try_read_key()`)
   - `Some(ms)`: Timeout-based waiting
   - `None`: Infinite blocking (not available on WASM)

#### Platform Compatibility Matrix

| API | Unix/Linux | Windows | WASM | Go Bindings |
|-----|------------|---------|------|-------------|
| `try_read_key()` | ✅ poll() | ✅ PeekConsoleInput | ✅ event queue | ✅ Recommended |
| `read_key_timeout(0)` | ✅ Same as above | ✅ Same as above | ✅ Same as above | ✅ Recommended |
| `read_key_timeout(ms)` | ✅ select() | ✅ WaitForSingleObject | ❌ Unsupported | ⚠️ Short timeouts only |
| `read_key_timeout(None)` | ✅ blocking read | ✅ ReadConsoleInput | ❌ Unsupported | ❌ Not recommended |

### Go Binding Strategy

- **Channel-Based High-Level API**: Provide idiomatic Go channels for key events
- **Avoid Infinite Blocking**: Discourage `read_key_timeout(None)` usage
- **Goroutine-Friendly**: Design around Go's concurrency patterns
- **Graceful Shutdown**: Support context cancellation and timeouts

```go
// Recommended Go API patterns
func (c *ConsoleInput) KeyEventChannel() <-chan KeyEvent
func (c *ConsoleInput) TryReadKey() (*KeyEvent, error)
func (c *ConsoleInput) ReadKeyWithTimeout(timeout time.Duration) (*KeyEvent, error)
```

## Implementation Tasks

### [COMPLETE] Phase 1: Core API Implementation (Task 4.4)

- [x] **Implement Unix `try_read_key()`** ✅ DONE
- [x] **Implement Unix `read_key_timeout()`** ✅ DONE  
- [x] **Update prompt.rs input() method** ✅ DONE
- [x] **Implement Windows `try_read_key()`** ✅ DONE
- [x] **Implement Windows `read_key_timeout()`** ✅ DONE
- [x] **Fix Escape key detection with KeyParser state management** ✅ DONE
- [x] **Fix raw mode output formatting with \\r\\n line endings** ✅ DONE

### [IN PROGRESS] Phase 2: WASM Integration

#### WASM Bridge Implementation

- [x] **Implement WASM `try_read_key()`** ✅ DONE
  - File: `crates/replkit-io/src/wasm.rs`
  - Non-blocking input queue polling
  - Proper error handling with ConsoleError::IoError
- [x] **Implement WASM `read_key_timeout()`** ✅ DONE
  - File: `crates/replkit-io/src/wasm.rs`
  - Support only `Some(0)` (delegate to `try_read_key()`)
  - Return `UnsupportedFeature` for blocking operations
  - Document WASM limitations clearly
- [x] **Go WASM Runtime Integration**
  - File: `crates/replkit-io/src/wasm.rs`
  - Export functions for Go wazero runtime
  - Handle key event marshaling from Go
  - Window size management from Go side

### [COMPLETE] Phase 3: Go Bindings

#### Go Interface Design

- [x] **Design Go ConsoleInput Interface**
  - File: `bindings/go/console_input.go`
  - Channel-based high-level API
  - Context support for cancellation
  - Idiomatic Go error handling
- [x] Update `bindings/go/_examples/key_input_debug/main.go`

## Implementation Details

### Error Handling Strategy

```rust
pub enum ConsoleError {
    IoError(String),
    UnsupportedFeature { feature: String, platform: String },
    TimeoutExpired,
    PlatformSpecific(Box<dyn std::error::Error + Send + Sync>),
}
```

### Platform-Specific Optimizations

#### Unix/Linux
- Use `epoll` on Linux for better performance with multiple file descriptors
- Support both termios and raw mode configurations
- Handle signal interruption gracefully

#### Windows
- Distinguish between legacy and VT console modes
- Handle Unicode input properly
- Support Windows-specific modifier keys

#### WASM
- Handle browser security restrictions
- Support mobile browser touch events

### Performance Considerations

- **Memory Management**: Minimize allocations in hot paths
- **System Call Optimization**: Batch operations where possible
- **Caching**: Cache platform capabilities and configuration
- **Threading**: Consider background polling threads for better responsiveness

## Window Size Management and Re-rendering

### Design Philosophy for Window Resize Handling

#### Responsibility Separation

```
Go Application Layer          ← High-level logic, prompt management
     ↓ (size changes)
Go Console Wrapper           ← Platform-specific size detection (SIGWINCH)
     ↓ (WASM calls)  
Rust Rendering Engine        ← Cross-platform rendering logic, layout calculation
     ↓ (platform output)
Platform Console Output      ← Raw terminal I/O
```

#### Go Side: Size Detection and Notification

- **SIGWINCH Signal Monitoring**: Use Go's `os/signal` to detect terminal resize
- **Channel-Based Communication**: Provide `MonitorWindowSize(ctx) <-chan WindowSize` for reactive patterns
- **Immediate Notification**: Forward size changes to Rust renderer via WASM calls
- **Platform Optimization**: Use `golang.org/x/term.GetSize()` for accurate size detection

#### Rust Side: Rendering and Layout Logic

- **Unified Rendering Logic**: All layout calculations performed in Rust for consistency
- **Automatic Re-layout**: Window size changes trigger cursor position recalculation
- **Cross-Platform Compatibility**: Same rendering logic works across Unix/Windows/WASM
- **Performance**: Efficient cursor positioning with terminal coordinate considerations

### Implementation Tasks for Window Management

#### Phase 1: Basic Window Size Integration

- [ ] **Go Console Wrapper with Size Monitoring**
  - File: `bindings/go/console.go`
  - Implement `MonitorWindowSize()` with SIGWINCH detection
  - Create `Console` struct combining input and rendering
  - Handle graceful shutdown and signal cleanup

- [ ] **WASM Renderer Bridge**
  - File: `bindings/go/wasm_renderer.go`
  - Implement `WasmRenderer` with size notification methods
  - Create `SetWindowSize()` function for Rust communication
  - Handle JSON serialization for complex render data

- [ ] **Rust WASM Renderer Implementation**
  - File: `crates/replkit-wasm/src/renderer.rs`
  - Implement `set_window_size()` export function
  - Create `render_prompt()` with cursor position calculation
  - Handle automatic re-layout on size changes

#### Phase 2: Advanced Rendering Features

- [ ] **Multi-line Prompt Support**
  - Handle prompts that span multiple lines
  - Implement word wrapping with window width consideration
  - Calculate cursor position across wrapped lines

- [ ] **Scroll Management**
  - Implement scrolling for buffers longer than terminal height
  - Handle prompt positioning with scroll offset
  - Optimize redraw for minimal flicker

- [ ] **Layout Optimization**
  - Implement differential rendering (only redraw changed areas)
  - Add terminal capability detection for optimization
  - Handle various terminal types and their quirks

### API Design Examples

#### Go Application Usage

```go
func main() {
    ctx := context.Background()
    console, err := replkit.NewConsole()
    if err != nil {
        log.Fatal(err)
    }
    
    // Monitor window size changes
    go func() {
        for size := range console.MonitorWindowSize(ctx) {
            log.Printf("Resized to %dx%d", size.Columns, size.Rows)
            // Automatic re-rendering triggered internally
        }
    }()
    
    // Main application loop
    for {
        key := <-console.KeyEvents(ctx)
        updateBuffer(key)
        console.Render(prompt, buffer, cursor) // Rust handles layout
    }
}
```

#### Rust WASM Interface

```rust
#[no_mangle]
pub extern "C" fn set_window_size(columns: u16, rows: u16) {
    // Update internal window size
    // Trigger re-layout of current prompt
    // Handle cursor position recalculation
}

#[no_mangle] 
pub extern "C" fn render_prompt(data_ptr: u32, data_len: u32) -> u32 {
    // Parse JSON render data from Go
    // Calculate multi-line layout with current window size
    // Render with proper cursor positioning
    // Return success/error status
}
```

### Cross-Platform Considerations

#### Unix/Linux
- Use `SIGWINCH` signal for resize detection
- Handle terminal escape sequences for cursor control
- Support various terminal emulators (xterm, gnome-terminal, etc.)

#### Windows
- Use Windows Console API for size detection
- Handle both legacy console and Windows Terminal
- Support UTF-8 and legacy code page configurations

## Future Enhancements

### Advanced Input Features

- [ ] **Mouse Event Support**
  - Extend API for mouse input
  - Handle click, scroll, and movement events
  - Cross-platform coordinate mapping

- [ ] **Bracketed Paste Support**
  - Detect and handle bracketed paste mode
  - Distinguish between typed and pasted content
  - Security considerations for paste content

- [ ] **Dynamic Terminal Feature Detection**
  - Runtime detection of terminal capabilities
  - Adaptive rendering based on available features
  - Fallback modes for limited terminals

### API Refinements

- [ ] **Input Event Filtering**
  - Configurable key event filtering
  - Focus management for complex applications
  - Input validation and sanitization

- [ ] **Performance Profiling**
  - Benchmark different polling strategies
  - Optimize for specific use cases
  - Memory usage profiling

## Notes

- The dual API approach (`try_read_key()` vs `read_key_timeout()`) provides clear intent while allowing implementation optimization
- WASM constraints drive the overall design toward non-blocking patterns
- Go bindings will emphasize channel-based patterns over direct API mapping
- Platform-specific optimizations are hidden behind the unified API
- Error handling distinguishes between platform limitations and actual errors

## Timeline Estimate

- **Phase 1 (Core)**: 1-2 weeks
- **Phase 2 (WASM)**: 1 week  
- **Phase 3 (Go)**: 1 week
- **Phase 4 (Testing)**: 1 week

**Total Estimated Time**: 4-5 weeks for complete implementation
