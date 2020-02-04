use tokio::io::{self, BufReader, BufWriter};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout};

#[derive(Debug)]
pub struct LocalProc {
    pub id: u32,
    pub stdin: Option<BufWriter<ChildStdin>>,
    pub stdout: Option<BufReader<ChildStdout>>,
    pub stderr: Option<BufReader<ChildStderr>>,
    child: Child,
}

impl LocalProc {
    pub async fn kill(mut self) -> io::Result<std::process::Output> {
        // Kill the process, ignoring any error (in case it already exited)
        //
        // NOTE: Must block wait for child to exit, otherwise seems to sit around
        self.child.kill()?;

        self.child.wait_with_output().await
    }
}

impl From<Child> for LocalProc {
    fn from(mut child: Child) -> Self {
        Self {
            id: child.id(),
            stdin: child.stdin.take().map(BufWriter::new),
            stdout: child.stdout.take().map(BufReader::new),
            stderr: child.stderr.take().map(BufReader::new),
            child,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::process::Command;

    #[tokio::test]
    async fn kill_should_kill_child() {
        let child = Command::new("sleep").arg("1000").spawn().unwrap();

        let local_proc = LocalProc::from(child);
        let output = local_proc.kill().await.unwrap();

        assert!(!output.status.success(), "Process was not terminated");
    }
}
