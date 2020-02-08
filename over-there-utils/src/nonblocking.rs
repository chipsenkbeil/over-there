use std::io;
use std::process::{ChildStderr, ChildStdin, ChildStdout};

pub fn child_stdin_set_nonblocking(child_stdin: &ChildStdin) -> io::Result<()> {
    #[cfg(unix)]
    unix::set_nonblocking(child_stdin)?;

    #[cfg(windows)]
    windows::set_nonblocking(child_stdin)?;

    Ok(())
}

pub fn child_stdout_set_nonblocking(child_stdout: &ChildStdout) -> io::Result<()> {
    #[cfg(unix)]
    unix::set_nonblocking(child_stdout)?;

    #[cfg(windows)]
    windows::set_nonblocking(child_stdout)?;

    Ok(())
}

pub fn child_stderr_set_nonblocking(child_stderr: &ChildStderr) -> io::Result<()> {
    #[cfg(unix)]
    unix::set_nonblocking(child_stderr)?;

    #[cfg(windows)]
    windows::set_nonblocking(child_stderr)?;

    Ok(())
}

#[cfg(unix)]
pub mod unix {
    use libc;
    use std::io;
    use std::os::unix;

    /// Sets the file descriptor as non-blocking
    pub fn set_nonblocking(x: &impl unix::io::AsRawFd) -> io::Result<()> {
        fd_set_nonblocking(x.as_raw_fd(), true)
    }

    #[cfg(target_os = "linux")]
    /// Lifted from https://github.com/rust-lang/rust/blob/3982d3514efbb65b3efac6bb006b3fa496d16663/src/libstd/sys/unix/fd.rs#L191
    fn fd_set_nonblocking(fd: unix::io::RawFd, nonblocking: bool) -> io::Result<()> {
        unsafe {
            let v = nonblocking as libc::c_int;
            cvt(libc::ioctl(fd, libc::FIONBIO, &v))?;
            Ok(())
        }
    }

    #[cfg(not(target_os = "linux"))]
    /// Lifted from https://github.com/rust-lang/rust/blob/3982d3514efbb65b3efac6bb006b3fa496d16663/src/libstd/sys/unix/fd.rs#L200
    fn fd_set_nonblocking(fd: unix::io::RawFd, nonblocking: bool) -> io::Result<()> {
        unsafe {
            let previous = cvt(libc::fcntl(fd, libc::F_GETFL))?;
            let new = if nonblocking {
                previous | libc::O_NONBLOCK
            } else {
                previous & !libc::O_NONBLOCK
            };
            if new != previous {
                cvt(libc::fcntl(fd, libc::F_SETFL, new))?;
            }
            Ok(())
        }
    }

    fn cvt(code: i32) -> io::Result<i32> {
        if code == -1 {
            Err(io::Error::last_os_error())
        } else {
            Ok(code)
        }
    }
}

#[cfg(windows)]
pub mod windows {
    use std::os::windows::io;

    pub fn set_nonblocking(x: &impl io::AsRawHandle) {
        handle_set_nonblocking(x.as_raw_handle())
    }

    fn handle_set_nonblocking(handle: io::RawHandle) {
        unimplemented!();
    }
}
