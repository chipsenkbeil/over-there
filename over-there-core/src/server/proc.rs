use std::process::Child;

#[derive(Debug)]
pub struct LocalProc {
    pub(crate) id: u32,
    pub child: Child,
}

impl From<Child> for LocalProc {
    fn from(child: Child) -> Self {
        Self {
            id: child.id(),
            child,
        }
    }
}

impl Drop for LocalProc {
    fn drop(&mut self) {
        // Ignore any error (in case it already exited)
        self.child.kill().unwrap_or(());

        // Must block wait for child to exit, otherwise seems to sit around
        self.child.wait().ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};

    #[test]
    fn local_proc_drop_should_kill_child() {
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
