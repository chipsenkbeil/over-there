use over_there_utils::nonblocking;
use std::convert::TryFrom;
use std::io::{self, Read, Write};
use std::process::{Child, ExitStatus};

#[derive(Debug)]
pub struct LocalProc {
    id: u32,
    child: Child,
}

impl LocalProc {
    pub fn new(child: Child) -> io::Result<Self> {
        if let Some(stdin) = child.stdin.as_ref() {
            nonblocking::child_stdin_set_nonblocking(stdin)?;
        }

        if let Some(stdout) = child.stdout.as_ref() {
            nonblocking::child_stdout_set_nonblocking(stdout)?;
        }

        if let Some(stderr) = child.stderr.as_ref() {
            nonblocking::child_stderr_set_nonblocking(stderr)?;
        }

        Ok(Self {
            id: child.id(),
            child,
        })
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn is_capturing_stdout(&self) -> bool {
        self.child.stdout.is_some()
    }

    pub fn read_stdout(&mut self, data: &mut [u8]) -> io::Result<usize> {
        match &mut self.child.stdout {
            Some(reader) => reader.read(data),
            None => Err(io::Error::from(io::ErrorKind::BrokenPipe)),
        }
    }

    pub fn is_capturing_stderr(&self) -> bool {
        self.child.stderr.is_some()
    }

    pub fn read_stderr(&mut self, data: &mut [u8]) -> io::Result<usize> {
        match &mut self.child.stderr {
            Some(reader) => reader.read(data),
            None => Err(io::Error::from(io::ErrorKind::BrokenPipe)),
        }
    }

    /// Attempts to kill the process and then waits for it to exit
    pub fn kill_and_wait(mut self) -> io::Result<ExitStatus> {
        self.child.kill()?;

        self.child.wait()
    }
}

impl Write for LocalProc {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        match self.child.stdin.as_mut() {
            Some(stdin) => stdin.write(data),
            None => Err(io::Error::from(io::ErrorKind::BrokenPipe)),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.child.stdin.as_mut() {
            Some(stdin) => stdin.flush(),
            None => Err(io::Error::from(io::ErrorKind::BrokenPipe)),
        }
    }
}

impl TryFrom<Child> for LocalProc {
    type Error = io::Error;

    fn try_from(child: Child) -> Result<Self, Self::Error> {
        Self::new(child)
    }
}

impl TryFrom<io::Result<Child>> for LocalProc {
    type Error = io::Error;

    fn try_from(result: io::Result<Child>) -> Result<Self, Self::Error> {
        result.and_then(Self::try_from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use over_there_utils::exec;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    fn wait_for_stdout(local_proc: &mut LocalProc) -> Vec<u8> {
        // Asserting that we get out what we put in eventually
        let mut buf = [0; 1024];
        exec::loop_timeout_panic(Duration::from_millis(500), move || {
            let result = local_proc.read_stdout(&mut buf);
            match result {
                Err(x) if x.kind() == io::ErrorKind::WouldBlock => None,
                Err(x) => panic!("Unexpected error {}", x),
                Ok(size) => Some(buf[..size].to_vec()),
            }
        })
    }

    fn wait_for_stderr(local_proc: &mut LocalProc) -> Vec<u8> {
        // Asserting that we get out what we put in eventually
        let mut buf = [0; 1024];
        exec::loop_timeout_panic(Duration::from_millis(500), move || {
            let result = local_proc.read_stderr(&mut buf);
            match result {
                Err(x) if x.kind() == io::ErrorKind::WouldBlock => None,
                Err(x) => panic!("Unexpected error {}", x),
                Ok(size) => Some(buf[..size].to_vec()),
            }
        })
    }

    #[test]
    fn id_should_return_child_id() {
        let child = Command::new("sleep").arg("1").spawn().unwrap();
        let id = child.id();

        let local_proc = LocalProc::try_from(child).unwrap();
        assert_eq!(id, local_proc.id());
    }

    #[test]
    fn is_capturing_stdout_should_return_true_if_child_stdout_piped() {
        let child = Command::new("sleep")
            .arg("1")
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        let local_proc = LocalProc::try_from(child).unwrap();
        assert!(local_proc.is_capturing_stdout());
    }

    #[test]
    fn is_capturing_stdout_should_return_false_if_child_stdout_not_piped() {
        let child = Command::new("sleep").arg("1").spawn().unwrap();

        let local_proc = LocalProc::try_from(child).unwrap();
        assert!(!local_proc.is_capturing_stdout());
    }

    #[test]
    fn is_capturing_stderr_should_return_true_if_child_stderr_piped() {
        let child = Command::new("sleep")
            .arg("1")
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let local_proc = LocalProc::try_from(child).unwrap();
        assert!(local_proc.is_capturing_stderr());
    }

    #[test]
    fn is_capturing_stderr_should_return_false_if_child_stderr_not_piped() {
        let child = Command::new("sleep").arg("1").spawn().unwrap();

        let local_proc = LocalProc::try_from(child).unwrap();
        assert!(!local_proc.is_capturing_stderr());
    }

    #[test]
    fn read_stdout_should_fill_buffer_and_return_size_of_bytes_read_from_stdout_of_child() {
        let child = Command::new("echo")
            .arg("abc")
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        let mut local_proc = LocalProc::try_from(child).unwrap();
        let stdout = wait_for_stdout(&mut local_proc);
        assert_eq!(stdout, b"abc\n");
    }

    #[test]
    fn read_stdout_should_return_would_block_error_if_no_stdout_available() {
        let child = Command::new("echo").stdout(Stdio::piped()).spawn().unwrap();

        let mut local_proc = LocalProc::try_from(child).unwrap();

        let x = local_proc.read_stdout(&mut [0; 1024]).unwrap_err();
        assert_eq!(x.kind(), io::ErrorKind::WouldBlock);
    }

    #[test]
    fn read_stdout_should_return_error_if_stdout_not_piped_from_child() {
        let child = Command::new("echo")
            .arg("abc")
            .stdout(Stdio::null())
            .spawn()
            .unwrap();

        let mut local_proc = LocalProc::try_from(child).unwrap();

        let err = local_proc.read_stdout(&mut [0; 1024]).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::BrokenPipe);
    }

    #[test]
    fn read_stderr_should_fill_buffer_and_return_size_of_bytes_read_from_stderr_of_child() {
        let child = Command::new("cat")
            .arg("abc")
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let mut local_proc = LocalProc::try_from(child).unwrap();

        // TODO: This seems a bit hacky of a test; need a guaranteed way to
        //       output a specific string to stderr
        let stderr = wait_for_stderr(&mut local_proc);
        assert_eq!(stderr, b"cat: ");
    }

    #[test]
    fn read_stderr_should_return_would_block_error_if_no_stderr_available() {
        let child = Command::new("echo")
            .arg("abc")
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let mut local_proc = LocalProc::try_from(child).unwrap();

        let x = local_proc.read_stderr(&mut [0; 1024]).unwrap_err();
        assert_eq!(x.kind(), io::ErrorKind::WouldBlock);
    }

    #[test]
    fn read_stderr_should_return_error_if_stderr_not_piped_from_child() {
        let child = Command::new("echo")
            .arg("abc")
            .stdout(Stdio::null())
            .spawn()
            .unwrap();

        let mut local_proc = LocalProc::try_from(child).unwrap();

        let err = local_proc.read_stderr(&mut [0; 1024]).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::BrokenPipe);
    }

    #[test]
    fn write_should_send_bytes_to_stdin_of_child() {
        let child = Command::new("cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        let mut local_proc = LocalProc::try_from(child).unwrap();

        // NOTE: We must send a newline in order for command to send response
        local_proc.write_all(b"abc\n").unwrap();
        let stdout = wait_for_stdout(&mut local_proc);
        assert_eq!(stdout, b"abc\n");

        // NOTE: We must send a newline in order for command to send response
        local_proc.write_all(b"def\n").unwrap();
        let stdout = wait_for_stdout(&mut local_proc);
        assert_eq!(stdout, b"def\n");
    }

    #[test]
    fn write_should_return_error_if_stdin_not_piped_from_child() {
        let child = Command::new("echo")
            .arg("abc")
            .stdout(Stdio::null())
            .spawn()
            .unwrap();

        let mut local_proc = LocalProc::try_from(child).unwrap();
        assert_eq!(
            local_proc.write(&mut [0; 1024]).unwrap_err().kind(),
            io::ErrorKind::BrokenPipe
        );
    }

    #[test]
    fn flush_should_return_error_if_stdin_not_piped_from_child() {
        let child = Command::new("echo")
            .arg("abc")
            .stdout(Stdio::null())
            .spawn()
            .unwrap();

        let mut local_proc = LocalProc::try_from(child).unwrap();
        assert_eq!(
            local_proc.flush().unwrap_err().kind(),
            io::ErrorKind::BrokenPipe
        );
    }

    #[test]
    fn kill_should_kill_child() {
        let secs = 1;
        let child = Command::new("sleep")
            .arg(format!("{}", secs))
            .spawn()
            .unwrap();

        let local_proc = LocalProc::try_from(child).unwrap();

        let before = Instant::now();
        let exit_status = local_proc.kill_and_wait().unwrap();
        let elapsed = before.elapsed();

        assert!(!exit_status.success(), "Proc not terminated");
        assert!(
            elapsed < Duration::from_secs(secs),
            "Kill just waited instead of killing proc"
        );
    }
}
