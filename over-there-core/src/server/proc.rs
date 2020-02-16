use std::io;
use std::process::Output;
use tokio::process::Child;

#[derive(Debug)]
pub struct LocalProc {
    id: u32,
    inner: Child,
}

impl LocalProc {
    pub fn new(child: Child) -> Self {
        Self {
            id: child.id(),
            inner: child,
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn inner(&self) -> &Child {
        &self.inner
    }

    pub async fn write_stdin(&mut self, buf: &[u8]) -> io::Result<()> {
        use tokio::io::AsyncWriteExt;

        match self.inner.stdin.as_mut() {
            Some(stdin) => stdin.write_all(buf).await,
            None => Err(io::Error::from(io::ErrorKind::BrokenPipe)),
        }
    }

    pub async fn read_stdout(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        use tokio::io::AsyncReadExt;

        match self.inner.stdout.as_mut() {
            Some(stdout) => stdout.read(buf).await,
            None => Err(io::Error::from(io::ErrorKind::BrokenPipe)),
        }
    }

    pub async fn read_stderr(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        use tokio::io::AsyncReadExt;

        match self.inner.stderr.as_mut() {
            Some(stderr) => stderr.read(buf).await,
            None => Err(io::Error::from(io::ErrorKind::BrokenPipe)),
        }
    }

    pub async fn kill_and_wait(mut self) -> io::Result<Output> {
        self.inner.kill()?;
        self.inner.wait_with_output().await
    }
}
