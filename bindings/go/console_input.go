// Package replkit provides Go bindings for the replkit console I/O library.
// This package uses WASM runtime (wazero) to interface with the Rust implementation.
package replkit

import (
	"context"
	"fmt"
	"os"
	"os/signal"
	"sync"
	"syscall"
	"time"

	"golang.org/x/sys/unix"
)

// ConsoleInput provides cross-platform console input functionality
// using the replkit Rust library through WASM runtime.
type ConsoleInput struct {
	keyParser   *KeyParser
	fd          int
	origTermios unix.Termios
	inputChan   chan KeyEvent
	sigChan     chan os.Signal
	sizeChan    chan WindowSize
	ctx         context.Context
	cancel      context.CancelFunc
	mu          sync.Mutex
	rawMode     bool
	running     bool
}

// WindowSize represents terminal window dimensions.
type WindowSize struct {
	Columns int
	Rows    int
}

// NewConsoleInput creates a new ConsoleInput instance.
func NewConsoleInput(ctx context.Context) (*ConsoleInput, error) {
	parser, err := New(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to create key parser: %w", err)
	}

	// Open /dev/tty for raw input like go-prompt does
	fd, err := syscall.Open("/dev/tty", syscall.O_RDONLY, 0)
	if err != nil {
		parser.Close()
		return nil, fmt.Errorf("failed to open /dev/tty: %w", err)
	}

	inputCtx, cancel := context.WithCancel(ctx)
	c := &ConsoleInput{
		keyParser: parser,
		fd:        fd,
		inputChan: make(chan KeyEvent, 100),
		sigChan:   make(chan os.Signal, 1),
		sizeChan:  make(chan WindowSize, 1),
		ctx:       inputCtx,
		cancel:    cancel,
	}

	// Register signal handlers for window size changes
	signal.Notify(c.sigChan, syscall.SIGWINCH)

	// Start monitoring window size changes
	go c.monitorWindowSize()

	return c, nil
}

// EnableRawMode enables raw terminal mode using syscalls like go-prompt.
func (c *ConsoleInput) EnableRawMode() error {
	c.mu.Lock()
	defer c.mu.Unlock()

	if c.rawMode {
		return nil // Already in raw mode
	}

	// Save original termios
	if err := c.getOriginalTermios(); err != nil {
		return fmt.Errorf("failed to get original termios: %w", err)
	}

	// Set non-blocking mode
	if err := syscall.SetNonblock(c.fd, true); err != nil {
		return fmt.Errorf("failed to set non-blocking mode: %w", err)
	}

	// Set raw mode
	if err := c.setRaw(); err != nil {
		syscall.SetNonblock(c.fd, false) // Restore blocking mode on error
		return fmt.Errorf("failed to set raw mode: %w", err)
	}

	c.rawMode = true

	// Start reading input in a separate goroutine
	go c.readInput()

	return nil
}

// DisableRawMode disables raw terminal mode.
func (c *ConsoleInput) DisableRawMode() error {
	c.mu.Lock()
	defer c.mu.Unlock()

	if !c.rawMode {
		return nil // Not in raw mode
	}

	// Set blocking mode
	if err := syscall.SetNonblock(c.fd, false); err != nil {
		return fmt.Errorf("failed to set blocking mode: %w", err)
	}

	// Restore original termios
	if err := c.restore(); err != nil {
		return fmt.Errorf("failed to restore terminal mode: %w", err)
	}

	c.rawMode = false
	return nil
}

// getOriginalTermios saves the original terminal settings
func (c *ConsoleInput) getOriginalTermios() error {
	termios, err := unix.IoctlGetTermios(c.fd, unix.TIOCGETA)
	if err != nil {
		return err
	}
	c.origTermios = *termios
	return nil
}

// setRaw puts terminal into raw mode like go-prompt's SetRaw function
func (c *ConsoleInput) setRaw() error {
	termios := c.origTermios

	// Disable input flags
	termios.Iflag &^= syscall.IGNBRK | syscall.BRKINT | syscall.PARMRK |
		syscall.ISTRIP | syscall.INLCR | syscall.IGNCR |
		syscall.ICRNL | syscall.IXON

	// Disable local flags
	termios.Lflag &^= syscall.ECHO | syscall.ICANON | syscall.IEXTEN |
		syscall.ISIG | syscall.ECHONL

	// Disable control flags
	termios.Cflag &^= syscall.CSIZE | syscall.PARENB
	termios.Cflag |= syscall.CS8 // Set to 8-bit wide

	// Set control characters
	termios.Cc[syscall.VMIN] = 1
	termios.Cc[syscall.VTIME] = 0

	return unix.IoctlSetTermios(c.fd, unix.TIOCSETA, &termios)
}

// restore restores the original terminal settings
func (c *ConsoleInput) restore() error {
	return unix.IoctlSetTermios(c.fd, unix.TIOCSETA, &c.origTermios)
}

// TryReadKey attempts to read a key without blocking.
func (c *ConsoleInput) TryReadKey() (*KeyEvent, error) {
	select {
	case event := <-c.inputChan:
		return &event, nil
	default:
		return nil, nil
	}
}

// ReadKey reads a key with an optional timeout.
func (c *ConsoleInput) ReadKey(timeout time.Duration) (*KeyEvent, error) {
	if timeout == 0 {
		// Blocking read
		select {
		case event := <-c.inputChan:
			return &event, nil
		case <-c.ctx.Done():
			return nil, c.ctx.Err()
		}
	}

	// Read with timeout
	timer := time.NewTimer(timeout)
	defer timer.Stop()

	select {
	case event := <-c.inputChan:
		return &event, nil
	case <-timer.C:
		return nil, nil // Timeout
	case <-c.ctx.Done():
		return nil, c.ctx.Err()
	}
}

// WindowSizeChanges returns a channel that receives window size changes.
func (c *ConsoleInput) WindowSizeChanges() <-chan WindowSize {
	return c.sizeChan
}

// GetWindowSize returns the current terminal window size using ioctl.
func (c *ConsoleInput) GetWindowSize() (WindowSize, error) {
	ws, err := unix.IoctlGetWinsize(c.fd, unix.TIOCGWINSZ)
	if err != nil {
		return WindowSize{}, fmt.Errorf("failed to get terminal size: %w", err)
	}
	return WindowSize{Columns: int(ws.Col), Rows: int(ws.Row)}, nil
}

// readInput reads raw input from the file descriptor and parses it into key events.
func (c *ConsoleInput) readInput() {
	c.mu.Lock()
	c.running = true
	c.mu.Unlock()

	const maxReadBytes = 1024
	buffer := make([]byte, maxReadBytes)

	for {
		select {
		case <-c.ctx.Done():
			return
		default:
			n, err := syscall.Read(c.fd, buffer)
			if err != nil {
				if err == syscall.EAGAIN || err == syscall.EWOULDBLOCK {
					// No data available, sleep briefly and continue
					time.Sleep(10 * time.Millisecond)
					continue
				}
				// Other errors, continue reading
				continue
			}

			if n > 0 {
				// Parse the input bytes using KeyParser
				input := buffer[:n]
				events, err := c.keyParser.Feed(input)
				if err != nil {
					continue // Skip unparseable input
				}

				// Send all parsed events to the channel
				for _, event := range events {
					select {
					case c.inputChan <- event:
					case <-c.ctx.Done():
						return
					default:
						// Channel is full, drop oldest events
						select {
						case <-c.inputChan:
						default:
						}
						select {
						case c.inputChan <- event:
						case <-c.ctx.Done():
							return
						}
					}
				}
			}
		}
	}
}

// monitorWindowSize monitors terminal window size changes.
func (c *ConsoleInput) monitorWindowSize() {
	// Send initial size
	if size, err := c.GetWindowSize(); err == nil {
		select {
		case c.sizeChan <- size:
		case <-c.ctx.Done():
			return
		}
	}

	for {
		select {
		case <-c.sigChan:
			if size, err := c.GetWindowSize(); err == nil {
				select {
				case c.sizeChan <- size:
				case <-c.ctx.Done():
					return
				}
			}
		case <-c.ctx.Done():
			return
		}
	}
}

// Close cleans up resources and restores terminal state.
func (c *ConsoleInput) Close() error {
	c.cancel()

	c.mu.Lock()
	defer c.mu.Unlock()

	if c.rawMode {
		// Restore terminal settings
		syscall.SetNonblock(c.fd, false)
		c.restore()
		c.rawMode = false
	}

	// Close file descriptor
	if c.fd != 0 {
		syscall.Close(c.fd)
	}

	signal.Stop(c.sigChan)
	close(c.inputChan)
	close(c.sizeChan)

	if c.keyParser != nil {
		return c.keyParser.Close()
	}

	return nil
}

// IsRawMode returns true if the terminal is in raw mode.
func (c *ConsoleInput) IsRawMode() bool {
	c.mu.Lock()
	defer c.mu.Unlock()
	return c.rawMode
}
