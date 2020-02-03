use std::io::{self, BufReader, BufWriter};
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout};

#[derive(Debug)]
pub struct LocalProc {
    pub id: u32,
    pub stdin: Option<BufWriter<ChildStdin>>,
    pub stdout: Option<BufReader<ChildStdout>>,
    pub stderr: Option<BufReader<ChildStderr>>,
    child: Child,
}

impl LocalProc {
    pub fn kill(&mut self) -> io::Result<()> {
        // Kill the process, ignoring any error (in case it already exited)
        //
        // NOTE: Must block wait for child to exit, otherwise seems to sit around
        self.child
            .kill()
            .and_then(|_| self.child.wait())
            .map(|_| ())
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

impl Drop for LocalProc {
    fn drop(&mut self) {
        self.kill().unwrap_or_default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};

    #[test]
    fn kill_should_kill_child() {
        let child = Command::new("sleep").arg("1000").spawn().unwrap();

        let mut local_proc = LocalProc::from(child);
        local_proc.kill().unwrap();

        assert!(
            local_proc.child.try_wait().unwrap().is_some(),
            "Process is still running"
        );
    }

    #[test]
    fn drop_should_kill_child() {
        let child = Command::new("sleep").arg("1000").spawn().unwrap();
        let id = child.id();

        let local_proc = LocalProc::from(child);
        drop(local_proc);

        // TODO: This is Unix/Linux specific and will not work on Windows, so
        //       we'd need to provide a windows-oriented alternative using
        //       os cfg to have a cross-platform test
        let status = Command::new("kill")
            .arg("-0")
            .arg(id.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert!(!status.success(), "Process is still running");
    }
}
