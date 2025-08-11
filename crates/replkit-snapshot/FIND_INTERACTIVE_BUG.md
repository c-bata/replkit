# Interactive Application Bug Analysis

## 🎯 Priority TODO List

- [ ] CRITICAL Priority (Fix Immediately)
  - **Add debug output to drain_output()** - Identify exact blocking point in PTY read operations
  - **Test synchronous PTY read operations** - Bypass potential tokio async/await deadlock
  - **Verify PTY file descriptor setup** - Use strace/lsof to confirm PTY communication during hang

- [ ] HIGH Priority (Next Steps)  
  - **Check portable-pty timeout behavior** - Verify read_output_timeout actually respects timeout parameter
  - **Test stdin fd vs PTY slave fd mismatch** - Compare file descriptors in simple_prompt vs replkit-snapshot
  - **Implement raw PTY operations** - Bypass portable-pty library completely for testing
  - **Add signal handling monitoring** - Check for SIGCHLD and other signal interference

-️️ ️[️ ️]️ MEDIUM Priority (Investigation)
  - **Test alternative PTY libraries** - Try `pty` or `tokio-pty` crates as validation
  - **Check raw mode settings propagation** - Verify terminal settings between PTY master/slave
  - **Create minimal PTY test case** - Isolate issue outside of replkit-snapshot context

- [ ] LOW Priority (Long-term)
  - **Document PTY requirements** - Define compatibility requirements for target applications
  - **Add comprehensive PTY testing** - Prevent future regressions

---

# Interactive Application Bug Analysis

## 🚨 Critical Issue

`replkit-snapshot` hangs indefinitely when testing interactive applications like `simple_prompt`. The tool works perfectly with simple commands (`echo`) but fails with any interactive PTY-based application.

## 🔍 Current Symptoms

### Working Cases
- ✅ `echo "test"` - completes in ~100ms
- ✅ `simple_echo_test.yaml` - generates snapshots correctly

### Failing Cases
- ❌ `simple_prompt` - hangs at first `waitIdle` step
- ❌ Any `snapshot` step with interactive process - never completes
- ❌ Even immediate snapshot without waits - hangs indefinitely

### Observed Behavior
1. Process spawns successfully (`Process is running successfully`)
2. Execution begins but stops at first step involving PTY interaction
3. No timeout handling works - process must be force-killed
4. PTY read operations appear to block despite 10ms timeouts

## 📋 Bug Hypothesis Analysis

### Hypothesis 1: PTY Communication Mismatch
**Problem**: `simple_prompt` expects stdin but PTY provides different file descriptor

**Evidence**:
- `simple_prompt` uses `io::stdin().as_raw_fd()` (line 35 in unix.rs)
- `replkit-snapshot` communicates via PTY master/slave pair
- stdin fd ≠ PTY slave fd

**Verification Steps**:
```bash
# Test 1: Check if process receives PTY input
strace -e trace=read,write -p <simple_prompt_pid>

# Test 2: Compare fd numbers
lsof -p <simple_prompt_pid> | grep -E "(stdin|pty)"

# Test 3: Manual PTY test
echo "test" | /path/to/simple_prompt  # Should fail with "not a TTY"
```

### Hypothesis 2: Raw Mode Interference
**Problem**: PTY master and slave have conflicting terminal settings

**Evidence**:
- `simple_prompt` sets raw mode with `VMIN=0, VTIME=0` (non-blocking)
- PTY might not propagate these settings correctly
- Raw mode settings on slave vs master mismatch

**Verification Steps**:
```bash
# Test 1: Check terminal attributes
stty -a < /dev/pts/N  # Check PTY slave settings
stty -a < /dev/stdin  # Compare with stdin settings

# Test 2: Test raw mode directly
./test_pty_raw_mode.c  # Custom C program to test PTY raw mode
```

### Hypothesis 3: Event Loop Blocking
**Problem**: tokio async runtime blocks on PTY operations

**Evidence**:
- `drain_output()` uses `read_output_timeout()` 
- Even 10ms timeouts seem to block indefinitely
- Async/await chain might be deadlocking

**Verification Steps**:
```bash
# Test 1: Add debug prints to drain_output
# Test 2: Try synchronous version without tokio
# Test 3: Check if portable-pty has blocking issues
```

### Hypothesis 4: portable-pty Library Bug
**Problem**: `portable-pty` doesn't handle non-blocking reads correctly

**Evidence**:
- Library might not implement timeout correctly on Unix
- read_output_timeout() might ignore timeout parameter
- Issue specific to interactive applications

**Verification Steps**:
```bash
# Test 1: Test portable-pty examples directly
# Test 2: Implement raw PTY operations without library
# Test 3: Check portable-pty issue tracker
```

### Hypothesis 5: Signal Handling Issues
**Problem**: SIGCHLD or other signals interfere with PTY operations

**Evidence**:
- Interactive applications might send different signals
- PTY operations might be interrupted by signals
- Signal handlers not properly configured

**Verification Steps**:
```bash
# Test 1: Monitor signals
strace -e trace=signal <replkit-snapshot command>

# Test 2: Block/ignore specific signals
# Test 3: Check process tree with pstree
```

## 🧪 Systematic Debugging Approach

### Phase 1: Isolate the Problem
```bash
# Step 1: Confirm basic PTY functionality
cargo run -- run --cmd "cat /dev/null" --steps examples/minimal_test.yaml --compare examples/snapshots --update

# Step 2: Test with different interactive commands
cargo run -- run --cmd "read -p 'Enter: ' var; echo $var" --steps examples/debug_test.yaml --compare examples/snapshots --update

# Step 3: Test with non-raw-mode interactive app
cargo run -- run --cmd "bc -l" --steps examples/calculator_test.yaml --compare examples/snapshots --update
```

### Phase 2: Add Debugging Infrastructure
```rust
// Add to pty.rs
impl PtyManager {
    pub async fn debug_drain_output(&mut self, max_wait: Duration) -> Result<Vec<u8>> {
        println!("[DEBUG] drain_output called with timeout {:?}", max_wait);
        println!("[DEBUG] Process running: {}", self.is_process_running());
        
        let start = std::time::Instant::now();
        let result = self.read_output_timeout(&mut [0u8; 1], Duration::from_millis(1)).await;
        println!("[DEBUG] read_output_timeout(1ms) took {:?}, result: {:?}", start.elapsed(), result);
        
        // ... existing implementation
    }
}
```

### Phase 3: Alternative Implementation Test
```rust
// Test direct PTY operations without portable-pty
use std::os::unix::io::AsRawFd;
use libc;

fn test_raw_pty_read(master_fd: i32) -> std::io::Result<Vec<u8>> {
    let mut buffer = [0u8; 1024];
    let mut fds = libc::pollfd {
        fd: master_fd,
        events: libc::POLLIN,
        revents: 0,
    };
    
    let poll_result = unsafe { libc::poll(&mut fds, 1, 10) }; // 10ms timeout
    
    if poll_result > 0 {
        let bytes_read = unsafe {
            libc::read(master_fd, buffer.as_mut_ptr() as *mut libc::c_void, buffer.len())
        };
        // Handle result...
    }
    
    Ok(vec![])
}
```

## 🔧 Potential Fix Strategies

### Strategy 1: Fix PTY-stdin Communication
```rust
// Modify PtyManager to properly connect stdin/stdout
impl PtyManager {
    pub fn spawn_command_with_pty_redirect(&mut self, cmd_config: &CommandConfig) -> Result<()> {
        // Use dup2 to redirect stdin/stdout to PTY slave
        // Ensure proper file descriptor inheritance
    }
}
```

### Strategy 2: Implement Synchronous PTY Operations
```rust
// Replace async PTY operations with sync + threads
pub fn drain_output_sync(&mut self, max_wait: Duration) -> Result<Vec<u8>> {
    // Use blocking read with poll() for timeout
    // Avoid tokio async complications
}
```

### Strategy 3: Use Alternative PTY Library
```toml
# Try different PTY implementation
[dependencies]
pty = "0.2"  # Instead of portable-pty
# or
tokio-pty = "0.1"
```

### Strategy 4: Force Flush and Terminate Pattern
```rust
// After each snapshot, force process termination
impl StepExecutor {
    async fn capture_snapshot_and_terminate(&mut self, config: &SnapshotConfig) -> Result<Snapshot> {
        let snapshot = self.capture_snapshot(config).await?;
        
        // Terminate process after snapshot
        self.pty_manager.terminate()?;
        
        Ok(snapshot)
    }
}
```

## 🎯 Action Plan

### Immediate Actions (Priority 1)
1. **Add comprehensive debug output** to `drain_output()` and `read_output_timeout()`
2. **Test with synchronous read operations** instead of async
3. **Verify PTY file descriptor setup** with strace/lsof
4. **Test alternative PTY libraries** as quick validation

### Medium Term (Priority 2)
1. **Implement proper stdin/stdout redirection** for spawned processes
2. **Add signal handling** for better process control
3. **Create minimal PTY test case** outside of replkit context

### Long Term (Priority 3)
1. **Redesign PTY communication architecture** if needed
2. **Add comprehensive PTY compatibility testing**
3. **Document PTY requirements** for target applications

## 🏁 Success Criteria

- [ ] `simple_prompt` test completes within 5 seconds
- [ ] All interactive applications can be terminated properly
- [ ] PTY communication works reliably with raw-mode applications
- [ ] Debug output clearly shows where blocking occurs
- [ ] Alternative PTY implementations tested and compared

---

**This bug is blocking the core functionality of replkit-snapshot. All interactive testing depends on resolving this PTY communication issue.**