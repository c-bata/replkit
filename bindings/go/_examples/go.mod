module github.com/c-bata/replkit/examples

go 1.23.0

toolchain go1.24.6

require github.com/c-bata/replkit/bindings/go v0.0.0

require (
	github.com/tetratelabs/wazero v1.7.0 // indirect
	golang.org/x/sys v0.35.0 // indirect
)

// Use local bindings
replace github.com/c-bata/replkit/bindings/go => ../
