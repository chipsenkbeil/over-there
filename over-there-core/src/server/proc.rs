use std::io::{self, BufReader, Read, Write};
use std::process::{self, Child, ChildStdin};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

#[derive(Debug)]
struct LocalProcReader {
    handle: JoinHandle<()>,
    rx: mpsc::Receiver<(usize, Vec<u8>)>,
    flag: Arc<AtomicBool>,
}

impl LocalProcReader {
    pub fn spawn(reader: impl Read + Send + 'static, buf_size: usize) -> Self {
        let (tx, rx) = mpsc::channel();
        let flag = Arc::new(AtomicBool::new(true));
        let flag_thr = Arc::clone(&flag);
        let handle = thread::spawn(move || {
            let mut buf_reader = BufReader::new(reader);
            let mut buf = vec![0; buf_size];
            while flag_thr.load(Ordering::Acquire) {
                // This will block until data is available; alternatively,
                // could try to get underlying descriptor and mark as nonblocking
                // using os-specific flags
                if let Ok(size) = buf_reader.read(&mut buf) {
                    // If we fail to send back data, the connection is closed,
                    // and we can end the thread
                    if size > 0 && tx.send((size, buf[..size].to_vec())).is_err() {
                        break;
                    } else {
                        // Delay a little bit so we don't slam the CPU
                        thread::sleep(Duration::from_millis(1));
                    }
                }
            }
        });

        Self { handle, rx, flag }
    }

    pub fn try_read(&self, data: &mut [u8]) -> io::Result<usize> {
        match self.rx.try_recv() {
            Ok((size, d)) => {
                // If the provided data buffer is smaller than the bytes read,
                // we will chop off those extra bytes, otherwise, we ensure
                // that the provided buffer meets the size of the data exactly
                // by taking a slice of N bytes
                let size = std::cmp::min(size, data.len());
                data[..size].copy_from_slice(&d[..size]);
                Ok(size)
            }
            Err(mpsc::TryRecvError::Empty) => Err(io::Error::from(io::ErrorKind::WouldBlock)),
            Err(mpsc::TryRecvError::Disconnected) => {
                Err(io::Error::from(io::ErrorKind::BrokenPipe))
            }
        }
    }
}

impl Drop for LocalProcReader {
    fn drop(&mut self) {
        // Mark the thread to conclude
        self.flag.store(false, Ordering::Release);
    }
}

#[derive(Debug)]
pub struct LocalProc {
    id: u32,
    child: Child,
    stdin: Option<ChildStdin>,
    stdout_reader: Option<LocalProcReader>,
    stderr_reader: Option<LocalProcReader>,
}

impl LocalProc {
    pub const DEFAULT_BUF_SIZE: usize = 1024;

    /// Will spawn threads to handle blocking read of stdout/stderr
    pub fn new(mut child: Child, buf_size: usize) -> Self {
        Self {
            id: child.id(),
            stdin: child.stdin.take(),
            stdout_reader: child
                .stdout
                .take()
                .map(|x| LocalProcReader::spawn(x, buf_size)),
            stderr_reader: child
                .stderr
                .take()
                .map(|x| LocalProcReader::spawn(x, buf_size)),
            child,
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn is_capturing_stdout(&self) -> bool {
        self.stdout_reader.is_some()
    }

    pub fn read_stdout(&self, data: &mut [u8]) -> io::Result<usize> {
        match &self.stdout_reader {
            Some(reader) => reader.try_read(data),
            None => Err(io::Error::from(io::ErrorKind::BrokenPipe)),
        }
    }

    pub fn is_capturing_stderr(&self) -> bool {
        self.stderr_reader.is_some()
    }

    pub fn read_stderr(&self, data: &mut [u8]) -> io::Result<usize> {
        match &self.stderr_reader {
            Some(reader) => reader.try_read(data),
            None => Err(io::Error::from(io::ErrorKind::BrokenPipe)),
        }
    }

    pub fn kill(mut self) -> io::Result<process::Output> {
        // Kill the process, ignoring any error (in case it already exited)
        //
        // NOTE: Must block wait for child to exit, otherwise seems to sit around
        self.child.kill()?;

        self.child.wait_with_output()
    }
}

impl Write for LocalProc {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        match self.stdin.as_mut() {
            Some(stdin) => stdin.write(data),
            None => Err(io::Error::from(io::ErrorKind::BrokenPipe)),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.stdin.as_mut() {
            Some(stdin) => stdin.flush(),
            None => Err(io::Error::from(io::ErrorKind::BrokenPipe)),
        }
    }
}

impl From<Child> for LocalProc {
    fn from(child: Child) -> Self {
        Self::new(child, Self::DEFAULT_BUF_SIZE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use std::time::Instant;

    #[test]
    fn id_should_return_child_id() {
        unimplemented!();
    }

    #[test]
    fn is_capturing_stdout_should_return_true_if_child_stdout_piped() {
        unimplemented!();
    }

    #[test]
    fn is_capturing_stdout_should_return_false_if_child_stdout_not_piped() {
        unimplemented!();
    }

    #[test]
    fn is_capturing_stderr_should_return_true_if_child_stderr_piped() {
        unimplemented!();
    }

    #[test]
    fn is_capturing_stderr_should_return_false_if_child_stderr_not_piped() {
        unimplemented!();
    }

    #[test]
    fn read_stdout_should_return_size_of_bytes_read_from_stdout_of_child() {
        unimplemented!();
    }

    #[test]
    fn read_stdout_should_fill_buffer_with_stdout_from_child() {
        unimplemented!();
    }

    #[test]
    fn read_stdout_should_return_zero_bytes_and_not_block_if_no_stdout_available() {
        unimplemented!();
    }

    #[test]
    fn read_stdout_should_return_error_if_stdout_not_piped_from_child() {
        unimplemented!();
    }

    #[test]
    fn read_stderr_should_return_size_of_bytes_read_from_stderr_of_child() {
        unimplemented!();
    }

    #[test]
    fn read_stderr_should_fill_buffer_with_stderr_from_child() {
        unimplemented!();
    }

    #[test]
    fn read_stderr_should_return_zero_bytes_and_not_block_if_no_stderr_available() {
        unimplemented!();
    }

    #[test]
    fn read_stderr_should_return_error_if_stderr_not_piped_from_child() {
        unimplemented!();
    }

    #[test]
    fn write_should_return_size_of_bytes_written_to_stdin_of_child() {
        unimplemented!();
    }

    #[test]
    fn write_should_send_bytes_to_stdin_of_child() {
        unimplemented!();
    }

    #[test]
    fn write_should_return_error_if_stdin_not_piped_from_child() {
        unimplemented!();
    }

    #[test]
    fn flush_should_flush_stdin_of_child() {
        unimplemented!();
    }

    #[test]
    fn flush_should_return_error_if_stdin_not_piped_from_child() {
        unimplemented!();
    }

    #[test]
    fn kill_should_kill_child() {
        let secs = 1;
        let child = Command::new("sleep")
            .arg(format!("{}", secs))
            .spawn()
            .unwrap();

        let local_proc = LocalProc::from(child);

        let before = Instant::now();
        let output = local_proc.kill().unwrap();
        let elapsed = before.elapsed();

        assert!(!output.status.success(), "Process was not terminated");
        assert!(
            elapsed < Duration::from_secs(secs),
            "Kill just waited instead of killing proc!"
        );
    }
}
