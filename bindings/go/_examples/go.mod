module github.com/c-bata/prompt/examples

go 1.21

require (
	github.com/c-bata/prompt/bindings/go v0.0.0
	golang.org/x/term v0.15.0
)

require (
	github.com/tetratelabs/wazero v1.7.0 // indirect
	golang.org/x/sys v0.15.0 // indirect
)

// Use local bindings
replace github.com/c-bata/prompt/bindings/go => ../
