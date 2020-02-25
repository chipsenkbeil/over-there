use log::error;
use std::io;
use std::process::Output;
use std::sync::Arc;
use tokio::{process::Child, runtime::Handle, sync::Mutex, task};

#[derive(Copy, Clone, Debug)]
pub struct ExitStatus {
    pub id: u32,
    pub is_success: bool,
    pub exit_code: Option<i32>,
}

#[derive(Debug)]
pub struct LocalProc {
    id: u32,
    inner: Arc<Child>,
    exit_status: Arc<Mutex<Option<ExitStatus>>>,

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
            exit_status: Arc::new(Mutex::new(None)),
            supports_stdin: child.stdin.is_some(),
            supports_stdout: child.stdout.is_some(),
            supports_stderr: child.stderr.is_some(),
            inner: Arc::new(child),
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

    pub async fn exit_status(&self) -> Option<ExitStatus> {
        *self.exit_status.lock().await
    }

    /// Spawns io-processing task for stdout/stderr
    /// Will panic if not in tokio runtime
    pub fn spawn(mut self) -> Self {
        // Only spawn once
        if self.io_handle.is_some() {
            return self;
        }

        // TODO: Make child be Option<Child> so we can take it to move
        //       into the exit status check below; or, to support the
        //       kill and wait method, we need to use Arc<Mutex>>?
        //       but awaiting would lock indefinitely, so maybe we
        //       don't need the mutex? -- Arc doesn't seem to work on
        //       first glance
        //
        // TODO: Add new field that is stdin of Option<ChildStdin> that
        //       will be filled here via a take on the child so we don't
        //       need to keep around the child directly

        let handle = Handle::current();
        let id = self.id;

        let stdout = self.inner.stdout.take();
        let stderr = self.inner.stderr.take();

        let stdout_buf = Arc::clone(&self.stdout_buf);
        let stderr_buf = Arc::clone(&self.stderr_buf);

        let mut inner = Arc::clone(&self.inner);
        let exit_status = Arc::clone(&self.exit_status);
        let io_handle = handle.spawn(async move {
            let _ = tokio::join!(
                async {
                    let status = inner.await;
                    *exit_status.lock().await = Some(ExitStatus {
                        id,
                        is_success: status.is_ok(),
                        exit_code: status.ok().and_then(|s| s.code()),
                    });
                },
                async {
                    use tokio::io::AsyncReadExt;

                    if let Some(mut stdout) = stdout {
                        let mut buf = [0; 1024];

                        loop {
                            match stdout.read(&mut buf).await {
                                Ok(size) if size > 0 => {
                                    stdout_buf
                                        .lock()
                                        .await
                                        .extend_from_slice(&buf[..size]);
                                }
                                Ok(_) => (),
                                Err(x) => {
                                    error!("stdout reader died: {}", x);
                                    break;
                                }
                            }

                            // NOTE: Loop recovers too quickly from await on
                            //       read yielding size == 0, so yielding to
                            //       ensure that other tasks get a fair shake
                            task::yield_now().await;
                        }
                    }
                },
                async {
                    use tokio::io::AsyncReadExt;

                    if let Some(mut stderr) = stderr {
                        let mut buf = [0; 1024];

                        loop {
                            match stderr.read(&mut buf).await {
                                Ok(size) if size > 0 => {
                                    stderr_buf
                                        .lock()
                                        .await
                                        .extend_from_slice(&buf[..size]);
                                }
                                Ok(_) => (),
                                Err(x) => {
                                    error!("stderr reader died: {}", x);
                                    break;
                                }
                            }

                            // NOTE: Loop recovers too quickly from await on
                            //       read yielding size == 0, so yielding to
                            //       ensure that other tasks get a fair shake
                            task::yield_now().await;
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

    pub async fn read_stdout(&mut self) -> io::Result<Vec<u8>> {
        if self.supports_stdout {
            Ok(self.stdout_buf.lock().await.drain(..).collect())
        } else {
            Err(io::Error::from(io::ErrorKind::BrokenPipe))
        }
    }

    pub async fn read_stderr(&mut self) -> io::Result<Vec<u8>> {
        if self.supports_stderr {
            Ok(self.stderr_buf.lock().await.drain(..).collect())
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

                        task::yield_now().await;
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

    #[tokio::test]
    async fn test_read_stdout_should_return_an_error_if_not_piped() {
        let child = Command::new("echo")
            .arg("test")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();

        let mut local_proc = LocalProc::new(child);
        match local_proc.read_stdout().await {
            Ok(_) => {
                panic!("Unexpectedly succeeded in reading stdout not piped")
            }
            Err(x) => assert_eq!(x.kind(), io::ErrorKind::BrokenPipe),
        }
    }

    #[tokio::test]
    async fn test_read_stdout_should_return_empty_content_if_none_available() {
        let child = Command::new("echo")
            .arg("test")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();

        // NOTE: Not spawning so we can ensure that no content is available
        let mut local_proc = LocalProc::new(child);

        match local_proc.read_stdout().await {
            Ok(buf) => assert!(buf.is_empty()),
            Err(x) => panic!("Unexpected error: {}", x),
        }
    }

    #[tokio::test]
    async fn test_read_stdout_should_not_return_content_returned_previously() {
        let child = Command::new("echo")
            .arg("test")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();

        let mut local_proc = LocalProc::new(child).spawn();

        // Get first batch of bytes and discard
        assert!(
            !timeout(Duration::from_millis(10), async {
                loop {
                    match local_proc.read_stdout().await {
                        Ok(buf) => {
                            if !buf.is_empty() {
                                break buf;
                            }

                            // NOTE: The read above is too quick as it only awaits
                            //       for a lock, and thereby prevents switching
                            //       to another task -- yield to enable switching
                            task::yield_now().await;
                        }
                        Err(x) => panic!("Unexpected error: {}", x),
                    }
                }
            })
            .await
            .unwrap()
            .is_empty(),
            "Failed to get first batch of content"
        );

        // Assert second batch is empty
        assert!(
            local_proc.read_stdout().await.unwrap().is_empty(),
            "Unexpectedly got content when nothing should be left"
        );
    }

    #[tokio::test]
    async fn test_read_stdout_should_return_content_if_available() {
        let child = Command::new("echo")
            .arg("test")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();

        let mut local_proc = LocalProc::new(child).spawn();

        let buf = timeout(Duration::from_millis(10), async {
            loop {
                match local_proc.read_stdout().await {
                    Ok(buf) => {
                        if !buf.is_empty() {
                            break buf;
                        }

                        // NOTE: The read above is too quick as it only awaits
                        //       for a lock, and thereby prevents switching
                        //       to another task -- yield to enable switching
                        task::yield_now().await;
                    }
                    Err(x) => panic!("Unexpected error: {}", x),
                }
            }
        })
        .await
        .unwrap();

        assert_eq!(buf, b"test\n");
    }

    #[tokio::test]
    async fn test_read_stderr_should_return_an_error_if_not_piped() {
        let child = Command::new("rev")
            .arg("--aaa")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();

        let mut local_proc = LocalProc::new(child);
        match local_proc.read_stderr().await {
            Ok(_) => {
                panic!("Unexpectedly succeeded in reading stderr not piped")
            }
            Err(x) => assert_eq!(x.kind(), io::ErrorKind::BrokenPipe),
        }
    }

    #[tokio::test]
    async fn test_read_stderr_should_return_empty_content_if_none_available() {
        let child = Command::new("rev")
            .arg("--aaa")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        // NOTE: Not spawning so we can ensure that no content is available
        let mut local_proc = LocalProc::new(child);

        match local_proc.read_stderr().await {
            Ok(buf) => assert!(buf.is_empty()),
            Err(x) => panic!("Unexpected error: {}", x),
        }
    }

    #[tokio::test]
    async fn test_read_stderr_should_not_return_content_returned_previously() {
        let child = Command::new("rev")
            .arg("--aaa")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let mut local_proc = LocalProc::new(child).spawn();

        // Get first batch of bytes and discard
        assert!(
            !timeout(Duration::from_millis(10), async {
                loop {
                    match local_proc.read_stderr().await {
                        Ok(buf) => {
                            if !buf.is_empty() {
                                break buf;
                            }

                            // NOTE: The read above is too quick as it only awaits
                            //       for a lock, and thereby prevents switching
                            //       to another task -- yield to enable switching
                            task::yield_now().await;
                        }
                        Err(x) => panic!("Unexpected error: {}", x),
                    }
                }
            })
            .await
            .unwrap()
            .is_empty(),
            "Failed to get first batch of content"
        );

        // Assert second batch is empty
        assert!(
            local_proc.read_stderr().await.unwrap().is_empty(),
            "Unexpectedly got content when nothing should be left"
        );
    }

    #[tokio::test]
    async fn test_read_stderr_should_return_content_if_available() {
        let child = Command::new("rev")
            .arg("--aaa")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let mut local_proc = LocalProc::new(child).spawn();

        let buf = timeout(Duration::from_millis(10), async {
            loop {
                match local_proc.read_stderr().await {
                    Ok(buf) => {
                        if !buf.is_empty() {
                            break buf;
                        }

                        // NOTE: The read above is too quick as it only awaits
                        //       for a lock, and thereby prevents switching
                        //       to another task -- yield to enable switching
                        task::yield_now().await;
                    }
                    Err(x) => panic!("Unexpected error: {}", x),
                }
            }
        })
        .await
        .unwrap();

        assert!(buf.len() > 0);
    }

    #[tokio::test]
    async fn kill_and_wait_should_kill_and_return_process_result() {
        let child = Command::new("sleep")
            .arg("60")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();

        let local_proc = LocalProc::new(child).spawn();
        match local_proc.kill_and_wait().await {
            Ok(_) => (),
            Err(x) => panic!("Unexpected error: {}", x),
        }
    }
}
