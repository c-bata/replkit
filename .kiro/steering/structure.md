# Project Structure

## Organization Principles
- Clear separation between core logic, bindings, and user-facing interfaces
- Logical grouping of interactive prompt components (line editor, completer, renderer, etc.)
- Consistent naming conventions across modules and packages
- Compatibility with multi-language integration (Rust, Python, Go)
- Separation between engine (core) and frontend (bindings or host environment)

## Common Directory Layout
```
/
├── crates/                     # Workspace crates
│   ├── prompt-core/            # Core REPL engine logic (editor, history, prompt loop)
│   ├── prompt-buffer/          # Text buffer
│   ├── prompt-render/          # Terminal rendering (styling, ANSI control)
│   ├── prompt-keybind/         # Key bindings (emacs)
│   ├── prompt-wasm/            # 
│   └── prompt-pyo3/            # PyO3-based Python binding (build with maturin)
├── bindings/                   # Language-specific bindings
│   ├── python/                 # PyO3-based Python binding (build with maturin)
│   │   ├── src/                # Python module implementation
│   │   └── tests/              # Tests for Python bindings (pytest)
│   └── go/                     # Go binding via Wasm and wazero runtime
│       └── wasm/               # Wasm-compiled binaries and helpers
├── references/                 # Language-specific bindings
│   └── go-prompt/              # The source code of https://github.com/c-bata/go-prompt/
├── docs/                       # Developer and user documentation
├── scripts/                    # Dev tooling and automation scripts
└── .kiro/                      # Kiro-specific files
    └── steering/               # AI assistant guidance
```

## File Naming Conventions
- Use kebab-case for crate and directory names: `prompt-core/`
- Use snake_case for Rust source files: `line_editor.rs`, `key_map.rs`
- Use PascalCase for Rust struct and enum names: `PromptLoop`, `LineBuffer`
- Use snake_case for Python files: `bindings.rs`, `prompt_module.py`
- Use lowerCamelCase for Go files: `wasmRunner.go`, `inputHandler.go`
- Use descriptive names that convey function (e.g., `renderer.rs`, `completion.rs`)

## Code Organization
- `prompt-core` owns the REPL engine: buffer, cursor, key handling, completion logic
- `prompt-render` abstracts ANSI rendering, themes, cursor positioning, etc.
- `prompt-bindings` exposes safe interfaces for multi-language FFI
- Language bindings wrap `prompt-bindings` and expose idiomatic APIs
- `examples/` contains end-to-end usage in Rust, Python, and Go
- Keep logic modular: editor, renderer, history, completer should be swappable/testable
- Add integration tests to validate bindings match expected behavior
- Group tests close to the logic they validate; avoid monolithic test suites
- Use feature flags (`[features]`) to toggle optional components like macros or WASM
