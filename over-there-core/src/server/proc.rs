use log::error;
use std::io;
use std::process::Output;
use std::sync::Arc;
use tokio::{process::Child, runtime::Handle, sync::Mutex, task};

#[derive(Debug)]
pub struct LocalProc {
    id: u32,
    inner: Child,

    supports_stdin: bool,
    supports_stdout: bool,
    supports_stderr: bool,

    /// Handle to task that is processing stdout/stderr
    io_handle: Option<task::JoinHandle<()>>,

    /// Internal buffer of all stdout that has been acquired
    stdout_buf: Arc<Mutex<Vec<u8>>>,

    /// Internal buffer of all stderr that has been acquired
    stderr_buf: Arc<Mutex<Vec<u8>>>,
}

impl LocalProc {
    pub fn new(child: Child) -> Self {
        Self {
            id: child.id(),
            supports_stdin: child.stdin.is_some(),
            supports_stdout: child.stdout.is_some(),
            supports_stderr: child.stderr.is_some(),
            inner: child,
            io_handle: None,
            stdout_buf: Arc::new(Mutex::new(Vec::new())),
            stderr_buf: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn inner(&self) -> &Child {
        &self.inner
    }

    /// Spawns io-processing task for stdout/stderr
    /// Will panic if not in tokio runtime
    pub fn spawn(mut self) -> Self {
        // Only spawn once
        if self.io_handle.is_some() {
            return self;
        }

        let handle = Handle::current();

        let stdout = self.inner.stdout.take();
        let stderr = self.inner.stderr.take();

        let stdout_buf = Arc::clone(&self.stdout_buf);
        let stderr_buf = Arc::clone(&self.stderr_buf);

        let io_handle = handle.spawn(async move {
            let _ = tokio::join!(
                async {
                    use tokio::io::AsyncReadExt;

                    if let Some(mut stdout) = stdout {
                        let mut buf = [0; 1024];

                        loop {
                            match stdout.read(&mut buf).await {
                                Ok(size) => {
                                    stdout_buf
                                        .lock()
                                        .await
                                        .extend_from_slice(&buf[..size]);
                                }
                                Err(x) => {
                                    error!("stdout reader died: {}", x);
                                    break;
                                }
                            }
                        }
                    }
                },
                async {
                    use tokio::io::AsyncReadExt;

                    if let Some(mut stderr) = stderr {
                        let mut buf = [0; 1024];

                        loop {
                            match stderr.read(&mut buf).await {
                                Ok(size) => {
                                    stderr_buf
                                        .lock()
                                        .await
                                        .extend_from_slice(&buf[..size]);
                                }
                                Err(x) => {
                                    error!("stderr reader died: {}", x);
                                    break;
                                }
                            }
                        }
                    }
                }
            );
        });

        self.io_handle = Some(io_handle);

        self
    }

    pub async fn write_stdin(&mut self, buf: &[u8]) -> io::Result<()> {
        use tokio::io::AsyncWriteExt;

        match self.inner.stdin.as_mut() {
            Some(stdin) => {
                let mut result = stdin.write_all(buf).await;
                if result.is_ok() {
                    result = stdin.flush().await;
                }
                result
            }
            None => Err(io::Error::from(io::ErrorKind::BrokenPipe)),
        }
    }

    pub async fn read_stdout(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.supports_stdout {
            let stdout_buf = self.stdout_buf.lock().await;
            let size = std::cmp::min(buf.len(), stdout_buf.len());
            if size > 0 {
                buf.copy_from_slice(&stdout_buf[..size]);
            }
            Ok(size)
        } else {
            Err(io::Error::from(io::ErrorKind::BrokenPipe))
        }
    }

    pub async fn read_stderr(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.supports_stderr {
            let stderr_buf = self.stderr_buf.lock().await;
            let size = std::cmp::min(buf.len(), stderr_buf.len());
            if size > 0 {
                buf.copy_from_slice(&stderr_buf[..size]);
            }
            Ok(size)
        } else {
            Err(io::Error::from(io::ErrorKind::BrokenPipe))
        }
    }

    pub async fn kill_and_wait(mut self) -> io::Result<Output> {
        self.inner.kill()?;
        self.inner.wait_with_output().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;
    use std::time::Duration;
    use tokio::process::Command;
    use tokio::time::timeout;
    use tokio::{fs, io};

    #[tokio::test]
    async fn test_id_should_return_child_id() {
        let child = Command::new("cat")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();

        let id = child.id();
        let local_proc = LocalProc::new(child);
        assert_eq!(id, local_proc.id());
    }

    #[tokio::test]
    async fn test_write_stdin_should_return_an_error_if_not_piped() {
        let child = Command::new("cat")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();

        let mut local_proc = LocalProc::new(child);
        match local_proc.write_stdin(&[1, 2, 3]).await {
            Ok(_) => panic!("Successfully wrote to stdin when not piped"),
            Err(x) => assert_eq!(x.kind(), io::ErrorKind::BrokenPipe),
        }
    }

    #[tokio::test]
    async fn test_write_stdin_should_write_contents_to_process() {
        let f = tempfile::tempfile().unwrap();
        let child = Command::new("cat")
            .stdin(Stdio::piped())
            .stdout(f.try_clone().unwrap())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();

        let mut local_proc = LocalProc::new(child);
        match local_proc.write_stdin(b"test").await {
            Ok(_) => {
                match timeout(Duration::from_millis(10), async {
                    use std::io::SeekFrom;
                    use tokio::io::AsyncReadExt;
                    let mut f = fs::File::from_std(f);

                    loop {
                        let mut s = String::new();
                        f.seek(SeekFrom::Start(0)).await.unwrap();
                        f.read_to_string(&mut s).await.unwrap();
                        if !s.is_empty() {
                            break s;
                        }
                    }
                })
                .await
                {
                    Ok(s) => assert_eq!(s, "test", "Unexpected output"),
                    Err(x) => panic!("Failed to write to file: {}", x),
                }
            }
            Err(_) => panic!("Failed to write to stdin"),
        }
    }

    #[test]
    fn test_read_stdout_should_return_an_error_if_not_piped() {
        unimplemented!();
    }

    #[test]
    fn test_read_stdout_should_yield_zero_size_if_no_content_available() {
        unimplemented!();
    }

    #[test]
    fn test_read_stdout_should_write_content_to_buf_and_return_bytes_read() {
        unimplemented!();
    }

    #[test]
    fn test_read_stderr_should_return_an_error_if_not_piped() {
        unimplemented!();
    }

    #[test]
    fn test_read_stderr_should_yield_zero_size_if_no_content_available() {
        unimplemented!();
    }

    #[test]
    fn test_read_stderr_should_write_content_to_buf_and_return_bytes_read() {
        unimplemented!();
    }

    #[test]
    fn kill_and_wait_should_kill_and_return_process_result() {
        unimplemented!();
    }
}
