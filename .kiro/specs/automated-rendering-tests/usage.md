# Design Document — `replkit-snapshot`

## Overview

`replkit-snapshot` is a CLI tool for snapshot testing terminal-based applications.
It is designed to work seamlessly with **[replkit](https://github.com/c-bata/replkit)** — a Rust port of [go-prompt](https://github.com/c-bata/go-prompt) with multi-language bindings (Rust, Python, Go).
The main goal is to verify that equivalent interactive programs written for different language bindings produce identical terminal output given the same sequence of key inputs.

The tool launches a program inside a pseudo-terminal (PTY), sends predefined key events (including printable characters and special keys), captures the rendered screen after each step, and compares it with stored golden snapshots.
This makes it possible to ensure behavioral parity across language bindings and to catch regressions in rendering or input handling.

## Motivation

While `replkit` aims to provide a consistent interactive REPL experience across multiple languages, terminal rendering can differ due to language runtime, library bindings, or environment.
For example:

* A Rust example and its Python binding counterpart should display identical completion menus for the same input sequence.
* A Go example should behave exactly the same as the Rust version when pressing `Tab` or `Ctrl+C`.

By automating key input and snapshot comparison, we can:

* Detect subtle differences in terminal output across bindings.
* Prevent regressions when modifying `replkit` internals.
* Increase confidence that all bindings behave consistently in CI.

## Key Features

* **CLI-based** — can be invoked from any language's test suite (`go test`, `pytest`, `cargo test`).
* **PTY-based execution** — runs the target application inside a pseudo-terminal to simulate a real terminal environment.
* **Configurable window size** — ensures deterministic rendering across environments.
* **Scripted key sequences** — supports printable characters, special keys (`Tab`, `Enter`, `Ctrl+C`), modifiers, repeats, and delays.
* **Snapshot capture after each step** — stores the screen buffer as normalized text for comparison.
* **Golden file comparison** — compares actual output with expected output stored in snapshot files; supports `--update` mode to refresh them.
* **Text normalization** — strip ANSI codes, trim trailing spaces, mask dynamic values (timestamps, paths), and control Unicode width calculation.
* **Wait conditions** — `waitIdle`, `waitForRegex`, and `waitExit` to handle asynchronous rendering.
* **Deterministic output** — controlled environment variables (`LANG`, `TERM`) and stable Unicode width settings.

## CLI Usage Example

```bash
replkit-snapshot run \
  --cmd './my-prompt-app' \
  --workdir ./examples \
  --env LANG=en_US.UTF-8 \
  --winsize 100x30 \
  --steps steps.yaml \
  --compare ./__snapshots__ \
  --update
```

Options:

* `--cmd <string>`: Command to run (shell or exec array).
* `--workdir <path>`: Working directory.
* `--env KEY=VAL`: Environment variables.
* `--winsize <cols>x<rows>`: Terminal size.
* `--steps <file>`: Step definition file (YAML/JSON).
* `--compare <dir>`: Golden snapshots directory.
* `--update`: Overwrite golden snapshots with current output.
* `--timeout <duration>`: Global timeout.
* `--idle-wait <duration>`: Delay after input before capturing output.
* `--strip-ansi`: Remove ANSI codes from snapshot text (default on).
* `--mask <rule>`: Mask variable text like timestamps.

Exit codes:

* `0`: Success
* `1`: Snapshot mismatch
* `2`: Runtime error

## Step Definition File (`steps.yaml`)

```yaml
version: 1
command:
  exec: ["./my-prompt-app", "--mode", "demo"]
  workdir: "./examples"
  env:
    LANG: "en_US.UTF-8"
    TERM: "xterm-256color"
tty:
  cols: 100
  rows: 30

steps:
  - send: "hello"
  - send: ["Tab"]
  - waitIdle: "50ms"
  - snapshot:
      name: "after-complete"
      stripAnsi: true
  - send: ["Ctrl+C"]
  - waitExit: "1s"
```

**Supported key syntax**:

* `"abc"` → `a`, `b`, `c`
* `["Tab"]`, `["Enter"]`, `["Esc"]`, `["Left"]`
* `"Ctrl+C"`, `"Alt+F"`, `"Shift+Tab"`
* `"Left*3"` (repeat)
* `{"sleep": "30ms"}` (delay between inputs)

## Snapshot Comparison

* Golden snapshots stored as plain text files: `example.after-complete.snap.txt`
* Unified diff output on mismatch
* Optional normalization:

  * Strip ANSI escape sequences
  * Trim trailing spaces
  * Collapse or preserve spaces
  * Mask non-deterministic values

## Example in Go Test

```go
func TestPromptCompletion(t *testing.T) {
    cmd := exec.Command("replkit-snapshot", "run",
        "--cmd", "./my-prompt-app",
        "--steps", "testdata/steps.yaml",
        "--compare", "testdata/__snapshots__",
        "--winsize", "100x30",
        "--timeout", "5s",
    )
    out, err := cmd.CombinedOutput()
    if err != nil {
        t.Log(string(out))
        t.Fatal(err)
    }
}
```

## Minimum Viable Product Scope

1. Launch target process in a PTY with configurable window size.
2. Send scripted key sequences and delays.
3. Capture screen buffer after each step (using `termwiz` in Rust).
4. Normalize and save snapshots.
5. Compare with golden files; support `--update`.
