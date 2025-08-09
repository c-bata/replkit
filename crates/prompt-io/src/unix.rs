use std::io::{self};
use std::os::unix::io::AsRawFd;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};

use crate::{ConsoleError, ConsoleInput, ConsoleResult, KeyEvent, KeyParser};

struct RawModeGuard {
    stdin_fd: i32,
    original_termios: libc::termios,
    original_flags: i32,
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = libc::tcsetattr(self.stdin_fd, libc::TCSANOW, &self.original_termios);
            let _ = libc::fcntl(self.stdin_fd, libc::F_SETFL, self.original_flags);
        }
    }
}

pub struct UnixVtConsoleInput {
    stdin_fd: i32,
    raw_guard: Option<RawModeGuard>,
    running: Arc<AtomicBool>,
    wake_fds: (i32, i32), // (read, write)
    resize_cb: Arc<Mutex<Option<Box<dyn FnMut(u16, u16) + Send>>>>,
    key_cb: Arc<Mutex<Option<Box<dyn FnMut(KeyEvent) + Send>>>>,
    thread: Option<JoinHandle<()>>,
}

impl UnixVtConsoleInput {
    pub fn new() -> io::Result<Self> {
        // Create self-pipe for waking up poll on stop/resize checks
        let mut fds = [0i32; 2];
        if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        // Set read end non-blocking
        let flags = unsafe { libc::fcntl(fds[0], libc::F_GETFL) };
        if flags != -1 {
            unsafe { libc::fcntl(fds[0], libc::F_SETFL, flags | libc::O_NONBLOCK) };
        }

        Ok(Self {
            stdin_fd: io::stdin().as_raw_fd(),
            raw_guard: None,
            running: Arc::new(AtomicBool::new(false)),
            wake_fds: (fds[0], fds[1]),
            resize_cb: Arc::new(Mutex::new(None)),
            key_cb: Arc::new(Mutex::new(None)),
            thread: None,
        })
    }

    fn enter_raw_mode(fd: i32) -> io::Result<RawModeGuard> {
        let mut original_termios = unsafe { std::mem::zeroed() };
        if unsafe { libc::tcgetattr(fd, &mut original_termios) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let mut raw = original_termios;
        raw.c_lflag &= !(libc::ICANON | libc::ECHO | libc::ECHOE | libc::ECHOK | libc::ECHONL | libc::ISIG | libc::IEXTEN);
        raw.c_iflag &= !(libc::IXON | libc::IXOFF | libc::ICRNL | libc::INLCR | libc::IGNCR | libc::BRKINT | libc::PARMRK | libc::ISTRIP);
        raw.c_oflag &= !libc::OPOST;
        raw.c_cflag &= !libc::CSIZE;
        raw.c_cflag |= libc::CS8;
        raw.c_cc[libc::VMIN] = 0; // non-blocking
        raw.c_cc[libc::VTIME] = 0;
        if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &raw) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags == -1 { return Err(io::Error::last_os_error()); }
        if unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } == -1 {
            return Err(io::Error::last_os_error());
        }
        Ok(RawModeGuard { stdin_fd: fd, original_termios, original_flags: flags })
    }

    fn poll_loop(
        stdin_fd: i32,
        wake_read: i32,
        running: Arc<AtomicBool>,
        resize_cb: Arc<Mutex<Option<Box<dyn FnMut(u16, u16) + Send>>>>,
        key_cb: Arc<Mutex<Option<Box<dyn FnMut(KeyEvent) + Send>>>>,
    ) {
        let mut parser = KeyParser::new();

        // Initial window size to detect changes
        let mut last_size = Self::query_window_size().ok();
        if let Some((cols, rows)) = last_size {
            if let Ok(mut g) = resize_cb.lock() {
                if let Some(cb) = g.as_mut() {
                    (cb)(cols, rows);
                }
            }
        }

        loop {
            if !running.load(Ordering::Relaxed) {
                break;
            }

            // Prepare poll fds: stdin and wake pipe
            let mut fds = [
                libc::pollfd { fd: stdin_fd, events: libc::POLLIN, revents: 0 },
                libc::pollfd { fd: wake_read, events: libc::POLLIN, revents: 0 },
            ];
            let rc = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, 50) }; // 50ms timeout to check resize
            if rc < 0 {
                // Interrupted; continue
                continue;
            }

            // Drain wake pipe if signaled
            if fds[1].revents & libc::POLLIN != 0 {
                let mut buf = [0u8; 64];
                unsafe { libc::read(wake_read, buf.as_mut_ptr() as *mut _, buf.len()) };
            }

            // Read stdin if ready
            if fds[0].revents & libc::POLLIN != 0 {
                let mut buf = [0u8; 1024];
                loop {
                    let n = unsafe { libc::read(stdin_fd, buf.as_mut_ptr() as *mut _, buf.len()) };
                    if n <= 0 { break; }
                    let bytes = &buf[..n as usize];
                    let events = parser.feed(bytes);
                    if !events.is_empty() {
                        if let Ok(mut g) = key_cb.lock() {
                            if let Some(cb) = g.as_mut() {
                                for ev in events { (cb)(ev); }
                            }
                        }
                    }
                }
            }

            // Resize check
            if let Ok((cols, rows)) = Self::query_window_size() {
                match last_size {
                    Some((c, r)) if c == cols && r == rows => {}
                    _ => {
                        last_size = Some((cols, rows));
                        if let Ok(mut g) = resize_cb.lock() {
                            if let Some(cb) = g.as_mut() { (cb)(cols, rows); }
                        }
                    }
                }
            }
        }
    }

    fn query_window_size() -> io::Result<(u16, u16)> {
        let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
        if unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) } == -1 {
            return Err(io::Error::last_os_error());
        }
        Ok((ws.ws_col as u16, ws.ws_row as u16))
    }
}

impl ConsoleInput for UnixVtConsoleInput {
    fn enable_raw_mode(&mut self) -> ConsoleResult<()> {
        if self.raw_guard.is_some() { return Ok(()); }
        let guard = Self::enter_raw_mode(self.stdin_fd)?;
        self.raw_guard = Some(guard);
        Ok(())
    }

    fn get_window_size(&self) -> ConsoleResult<(u16, u16)> {
        Ok(Self::query_window_size()?)
    }

    fn set_resize_callback(&mut self, callback: Box<dyn FnMut(u16, u16) + Send>) {
        if let Ok(mut g) = self.resize_cb.lock() {
            *g = Some(callback);
        }
    }

    fn set_key_event_callback(&mut self, callback: Box<dyn FnMut(KeyEvent) + Send>) {
        if let Ok(mut g) = self.key_cb.lock() {
            *g = Some(callback);
        }
    }

    fn start_event_loop(&mut self) -> ConsoleResult<()> {
        if self.running.swap(true, Ordering::Relaxed) {
            return Err(ConsoleError::AlreadyRunning);
        }
        let stdin_fd = self.stdin_fd;
        let wake_read = self.wake_fds.0;
        let running = self.running.clone();
        let resize_cb = self.resize_cb.clone();
        let key_cb = self.key_cb.clone();
        self.thread = Some(thread::spawn(move || {
            Self::poll_loop(stdin_fd, wake_read, running, resize_cb, key_cb);
        }));
        Ok(())
    }

    fn stop_event_loop(&mut self) -> ConsoleResult<()> {
        if !self.running.swap(false, Ordering::Relaxed) {
            return Err(ConsoleError::NotRunning);
        }
        // Wake the poll by writing a byte
        let _ = unsafe { libc::write(self.wake_fds.1, &1u8 as *const _ as *const _, 1) };
        if let Some(h) = self.thread.take() { let _ = h.join(); }
        Ok(())
    }
}
