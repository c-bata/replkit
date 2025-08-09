# Go Key Parser Binding

This package provides Go bindings for the Rust-based key input parser using WebAssembly (WASM).

## Usage

```go
package main

import (
    "context"
    "fmt"
    "io/ioutil"
    
    keyparsing "github.com/c-bata/prompt/bindings/go"
)

func main() {
    ctx := context.Background()
    
    // Load the WASM binary
    wasmBytes, err := ioutil.ReadFile("path/to/prompt_wasm.wasm")
    if err != nil {
        panic(err)
    }
    
    // Create a new parser
    parser, err := keyparsing.NewKeyParser(ctx, wasmBytes)
    if err != nil {
        panic(err)
    }
    defer parser.Close()
    
    // Parse some input
    events, err := parser.Feed([]byte{0x1b, 0x5b, 0x41}) // Up arrow
    if err != nil {
        panic(err)
    }
    
    for _, event := range events {
        fmt.Printf("Key: %v, Raw: %v\n", event.Key, event.RawBytes)
    }
}
```

## Dependencies

- [wazero](https://github.com/tetratelabs/wazero): WebAssembly runtime for Go