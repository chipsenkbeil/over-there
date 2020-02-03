use crate::{
    msg::content::{
        io::{proc::*, IoErrorArgs},
        Content,
    },
    server::{action::ActionError, proc::LocalProc, state::ServerState},
};
use log::debug;
use std::io;
use std::process::{Command, Stdio};

pub fn do_exec_proc(
    state: &mut ServerState,
    args: &DoExecProcArgs,
    respond: impl FnOnce(Content) -> Result<(), ActionError>,
) -> Result<(), ActionError> {
    debug!("do_exec_proc: {:?}", args);

    unimplemented!();
}

pub fn do_write_stdin(
    state: &mut ServerState,
    args: &DoWriteStdinArgs,
    respond: impl FnOnce(Content) -> Result<(), ActionError>,
) -> Result<(), ActionError> {
    debug!("do_write_stdin: {:?}", args);

    unimplemented!();
}

pub fn do_get_stdout(
    state: &mut ServerState,
    args: &DoGetStdoutArgs,
    respond: impl FnOnce(Content) -> Result<(), ActionError>,
) -> Result<(), ActionError> {
    debug!("do_get_stdout: {:?}", args);

    unimplemented!();
}

pub fn do_get_stderr(
    state: &mut ServerState,
    args: &DoGetStderrArgs,
    respond: impl FnOnce(Content) -> Result<(), ActionError>,
) -> Result<(), ActionError> {
    debug!("do_get_stderr: {:?}", args);

    unimplemented!();
}

pub fn do_proc_kill(
    state: &mut ServerState,
    args: &DoProcKillArgs,
    respond: impl FnOnce(Content) -> Result<(), ActionError>,
) -> Result<(), ActionError> {
    debug!("do_proc_kill: {:?}", args);

    unimplemented!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn do_exec_proc_should_send_success_if_can_execute_process() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;

        do_exec_proc(
            &mut state,
            &DoExecProcArgs {
                command: String::from("sleep"),
                args: vec![String::from("1")],
                stdin: false,
                stdout: false,
                stderr: false,
            },
            |c| {
                content = Some(c);
                Ok(())
            },
        )
        .unwrap();

        match content.unwrap() {
            Content::ProcStarted(args) => {
                let proc = state.procs.get(&args.id).unwrap();
                assert_eq!(proc.id, args.id);
                assert_eq!(proc.child.id(), args.id);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[test]
    fn do_exec_proc_should_send_error_if_process_does_not_exist() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;

        do_exec_proc(
            &mut state,
            &DoExecProcArgs {
                command: String::from("<a><b><c>"),
                args: vec![],
                stdin: false,
                stdout: false,
                stderr: false,
            },
            |c| {
                content = Some(c);
                Ok(())
            },
        )
        .unwrap();

        match content.unwrap() {
            Content::IoError(args) => assert_eq!(args.error_kind, io::ErrorKind::NotFound),
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[test]
    fn do_exec_proc_should_send_error_if_do_not_have_permission_to_execute_process() {
        unimplemented!();
    }

    #[test]
    fn do_write_stdin_should_send_data_to_running_process() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;

        let id = 999;
        let input = b"test\n".to_vec();
        let child = Command::new("read").stdin(Stdio::piped()).spawn().unwrap();
        state.procs.insert(id, LocalProc { id, child });

        // Give process some time to start
        thread::sleep(Duration::from_millis(10));

        do_write_stdin(&mut state, &DoWriteStdinArgs { id, input }, |c| {
            content = Some(c);
            Ok(())
        })
        .unwrap();

        match content.unwrap() {
            Content::StdinWritten(_) => (),
            x => panic!("Bad content: {:?}", x),
        }

        assert_eq!(std::env::var("REPLY").unwrap(), "test");
    }

    #[test]
    fn do_write_stdin_should_send_error_if_process_exited() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;

        let id = 999;
        let input = b"test\n".to_vec();
        let child = Command::new("sleep").stdin(Stdio::piped()).spawn().unwrap();
        state.procs.insert(id, LocalProc { id, child });

        // Give process some time to run and complete
        thread::sleep(Duration::from_millis(10));

        do_write_stdin(&mut state, &DoWriteStdinArgs { id, input }, |c| {
            content = Some(c);
            Ok(())
        })
        .unwrap();

        match content.unwrap() {
            Content::IoError(IoErrorArgs { error_kind, .. }) => {
                assert_eq!(error_kind, io::ErrorKind::BrokenPipe);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[test]
    fn do_write_stdin_should_send_error_if_process_id_not_registered() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;

        do_write_stdin(
            &mut state,
            &DoWriteStdinArgs {
                id: 0,
                input: b"test\n".to_vec(),
            },
            |c| {
                content = Some(c);
                Ok(())
            },
        )
        .unwrap();

        match content.unwrap() {
            Content::IoError(IoErrorArgs { error_kind, .. }) => {
                assert_eq!(error_kind, io::ErrorKind::InvalidInput);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[test]
    fn do_get_stdout_should_send_contents_if_process_sent_stdout() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;

        let id = 999;
        let child = Command::new("echo")
            .arg("test")
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        state.procs.insert(id, LocalProc { id, child });

        // Give process some time to run and complete
        thread::sleep(Duration::from_millis(10));

        do_get_stdout(&mut state, &DoGetStdoutArgs { id }, |c| {
            content = Some(c);
            Ok(())
        })
        .unwrap();

        match content.unwrap() {
            Content::StdoutContents(StdoutContentsArgs { output }) => {
                assert_eq!(output, b"test");
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[test]
    fn do_get_stdout_should_send_empty_contents_if_process_has_no_stdout() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;

        let id = 999;
        let child = Command::new("sleep")
            .arg("1")
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        state.procs.insert(id, LocalProc { id, child });

        // Give process some time to start
        thread::sleep(Duration::from_millis(10));

        do_get_stdout(&mut state, &DoGetStdoutArgs { id }, |c| {
            content = Some(c);
            Ok(())
        })
        .unwrap();

        match content.unwrap() {
            Content::StdoutContents(StdoutContentsArgs { output }) => {
                assert!(output.is_empty());
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[test]
    fn do_get_stdout_should_send_error_if_process_id_not_registered() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;

        do_get_stdout(&mut state, &DoGetStdoutArgs { id: 0 }, |c| {
            content = Some(c);
            Ok(())
        })
        .unwrap();

        match content.unwrap() {
            Content::IoError(IoErrorArgs { error_kind, .. }) => {
                assert_eq!(error_kind, io::ErrorKind::InvalidInput);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[test]
    fn do_get_stderr_should_send_contents_if_process_sent_stderr() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;

        let id = 999;
        let child = Command::new("echo")
            .arg("test")
            .arg("1>&2")
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        state.procs.insert(id, LocalProc { id, child });

        // Give process some time to run and complete
        thread::sleep(Duration::from_millis(10));

        do_get_stderr(&mut state, &DoGetStderrArgs { id }, |c| {
            content = Some(c);
            Ok(())
        })
        .unwrap();

        match content.unwrap() {
            Content::StderrContents(StderrContentsArgs { output }) => {
                assert_eq!(output, b"test");
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[test]
    fn do_get_stderr_should_send_empty_contents_if_process_has_no_stderr() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;

        let id = 999;
        let child = Command::new("sleep")
            .arg("1")
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        state.procs.insert(id, LocalProc { id, child });

        // Give process some time to start
        thread::sleep(Duration::from_millis(10));

        do_get_stderr(&mut state, &DoGetStderrArgs { id }, |c| {
            content = Some(c);
            Ok(())
        })
        .unwrap();

        match content.unwrap() {
            Content::StderrContents(StderrContentsArgs { output }) => {
                assert!(output.is_empty());
            }
            x => panic!("Bad content: {:?}", x),
        }
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;

        let id = 999;
        let child = Command::new("sleep").arg("1").spawn().unwrap();
        state.procs.insert(id, LocalProc { id, child });

        // Give process some time to start
        thread::sleep(Duration::from_millis(10));

        do_get_stderr(&mut state, &DoGetStderrArgs { id }, |c| {
            content = Some(c);
            Ok(())
        })
        .unwrap();

        match content.unwrap() {
            Content::StderrContents(StderrContentsArgs { output }) => {
                assert!(output.is_empty());
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[test]
    fn do_get_stderr_should_send_error_if_process_id_not_registered() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;

        do_get_stderr(&mut state, &DoGetStderrArgs { id: 0 }, |c| {
            content = Some(c);
            Ok(())
        })
        .unwrap();

        match content.unwrap() {
            Content::IoError(IoErrorArgs { error_kind, .. }) => {
                assert_eq!(error_kind, io::ErrorKind::InvalidInput);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[test]
    fn do_proc_kill_should_send_exit_status_after_killing_process() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;

        let id = 999;
        let child = Command::new("sleep").arg("10").spawn().unwrap();
        state.procs.insert(id, LocalProc { id, child });

        // Give process some time to start
        thread::sleep(Duration::from_millis(10));

        do_proc_kill(&mut state, &DoProcKillArgs { id }, |c| {
            content = Some(c);
            Ok(())
        })
        .unwrap();

        match content.unwrap() {
            Content::ProcStatus(ProcStatusArgs {
                id: exit_id,
                is_alive,
                ..
            }) => {
                assert_eq!(exit_id, id);
                assert!(!is_alive);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[test]
    fn do_proc_kill_should_send_error_if_process_already_exited() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;

        let id = 999;
        let child = Command::new("echo").spawn().unwrap();
        state.procs.insert(id, LocalProc { id, child });

        // Give process some time to run and complete
        thread::sleep(Duration::from_millis(10));

        do_proc_kill(&mut state, &DoProcKillArgs { id }, |c| {
            content = Some(c);
            Ok(())
        })
        .unwrap();

        match content.unwrap() {
            Content::IoError(IoErrorArgs { error_kind, .. }) => {
                assert_eq!(error_kind, io::ErrorKind::InvalidInput)
            }
            x => panic!("Bad content: {:?}", x),
        }
    }
}
