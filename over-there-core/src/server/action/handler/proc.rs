use crate::{
    msg::content::{
        io::{proc::*, IoErrorArgs},
        Content,
    },
    server::{action::ActionError, proc::LocalProc, state::ServerState},
};
use log::debug;
use std::convert::TryFrom;
use std::io;
use std::process::{Command, Stdio};

pub async fn do_exec_proc<F>(
    state: &mut ServerState,
    args: &DoExecProcArgs,
    respond: F,
) -> Result<(), ActionError>
where
    F: FnOnce(Content) -> Result<(), ActionError>,
{
    debug!("do_exec_proc: {:?}", args);
    let DoExecProcArgs {
        command,
        args,
        stdin,
        stdout,
        stderr,
    } = args;

    let make_pipe = |yes| if yes { Stdio::piped() } else { Stdio::null() };

    match LocalProc::try_from(
        Command::new(command)
            .args(args)
            .stdin(make_pipe(*stdin))
            .stdout(make_pipe(*stdout))
            .stderr(make_pipe(*stderr))
            .spawn(),
    ) {
        Ok(local_proc) => {
            let id = local_proc.id();
            state.procs.insert(id, local_proc);
            respond(Content::ProcStarted(ProcStartedArgs { id }))
        }
        Err(x) => respond(Content::IoError(From::from(x))),
    }
}

pub async fn do_write_stdin<F>(
    state: &mut ServerState,
    args: &DoWriteStdinArgs,
    respond: F,
) -> Result<(), ActionError>
where
    F: FnOnce(Content) -> Result<(), ActionError>,
{
    debug!("do_write_stdin: {:?}", args);

    match state.procs.get_mut(&args.id) {
        Some(local_proc) => {
            use std::io::Write;

            let mut result = local_proc.write_all(&args.input);
            if result.is_ok() {
                result = local_proc.flush();
            }

            match result {
                Ok(_) => respond(Content::StdinWritten(StdinWrittenArgs)),
                Err(x) => respond(Content::IoError(From::from(x))),
            }
        }
        None => respond(Content::IoError(IoErrorArgs::invalid_proc_id(args.id))),
    }
}

pub async fn do_get_stdout<F>(
    state: &mut ServerState,
    args: &DoGetStdoutArgs,
    respond: F,
) -> Result<(), ActionError>
where
    F: FnOnce(Content) -> Result<(), ActionError>,
{
    debug!("do_get_stdout: {:?}", args);

    match state.procs.get_mut(&args.id) {
        Some(local_proc) => {
            let mut buf = [0; 1024];
            match local_proc.read_stdout(&mut buf) {
                Ok(size) => respond(Content::StdoutContents(StdoutContentsArgs {
                    output: buf[..size].to_vec(),
                })),
                Err(x) if x.kind() == io::ErrorKind::WouldBlock => {
                    respond(Content::StdoutContents(StdoutContentsArgs {
                        output: vec![],
                    }))
                }
                Err(x) => respond(Content::IoError(From::from(x))),
            }
        }
        None => respond(Content::IoError(IoErrorArgs::invalid_proc_id(args.id))),
    }
}

pub async fn do_get_stderr<F>(
    state: &mut ServerState,
    args: &DoGetStderrArgs,
    respond: F,
) -> Result<(), ActionError>
where
    F: FnOnce(Content) -> Result<(), ActionError>,
{
    debug!("do_get_stderr: {:?}", args);

    match state.procs.get_mut(&args.id) {
        Some(local_proc) => {
            let mut buf = [0; 1024];
            match local_proc.read_stderr(&mut buf) {
                Ok(size) => respond(Content::StderrContents(StderrContentsArgs {
                    output: buf[..size].to_vec(),
                })),
                Err(x) if x.kind() == io::ErrorKind::WouldBlock => {
                    respond(Content::StderrContents(StderrContentsArgs {
                        output: vec![],
                    }))
                }
                Err(x) => respond(Content::IoError(From::from(x))),
            }
        }
        None => respond(Content::IoError(IoErrorArgs::invalid_proc_id(args.id))),
    }
}

pub async fn do_kill_proc<F>(
    state: &mut ServerState,
    args: &DoKillProcArgs,
    respond: F,
) -> Result<(), ActionError>
where
    F: FnOnce(Content) -> Result<(), ActionError>,
{
    debug!("do_kill_proc: {:?}", args);

    match state.procs.remove(&args.id) {
        // NOTE: We are killing and then WAITING for the process to die, which
        //       would block, but seems to be required in order to properly
        //       have the process clean up -- try_wait doesn't seem to work
        Some(local_proc) => match local_proc.kill_and_wait() {
            Ok(exit_status) => respond(Content::ProcStatus(ProcStatusArgs {
                id: args.id,
                is_alive: false,
                exit_code: exit_status.code(),
            })),
            Err(x) => respond(Content::IoError(From::from(x))),
        },
        None => respond(Content::IoError(IoErrorArgs::invalid_proc_id(args.id))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use over_there_utils::exec;
    use std::convert::TryFrom;
    use std::io;
    use std::process::Stdio;
    use std::thread;
    use std::time::Duration;

    #[tokio::test]
    async fn do_exec_proc_should_send_success_if_can_execute_process() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;

        do_exec_proc(
            &mut state,
            &DoExecProcArgs {
                command: String::from("rev"),
                args: vec![String::from("test")],
                stdin: false,
                stdout: false,
                stderr: false,
            },
            |c| {
                content = Some(c);
                Ok(())
            },
        )
        .await
        .unwrap();

        match content.unwrap() {
            Content::ProcStarted(args) => {
                let proc = state.procs.get(&args.id).unwrap();
                assert_eq!(proc.id(), args.id);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_exec_proc_should_send_error_if_process_does_not_exist() {
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
        .await
        .unwrap();

        match content.unwrap() {
            Content::IoError(args) => assert_eq!(args.error_kind, io::ErrorKind::NotFound),
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_write_stdin_should_send_data_to_running_process() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;

        let id = 999;
        let input = b"test\n".to_vec();
        let child = Command::new("cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        state.procs.insert(id, LocalProc::try_from(child).unwrap());

        // Give process some time to start
        thread::sleep(Duration::from_millis(10));

        do_write_stdin(&mut state, &DoWriteStdinArgs { id, input }, |c| {
            content = Some(c);
            Ok(())
        })
        .await
        .unwrap();

        let output = {
            let local_proc = state.procs.get_mut(&id).unwrap();

            let mut buf = [0; 1024];
            exec::loop_timeout_panic(Duration::from_millis(500), move || {
                let result = local_proc.read_stdout(&mut buf);
                match result {
                    Err(x) if x.kind() == io::ErrorKind::WouldBlock => None,
                    Err(x) => panic!("Unexpected error {}", x),
                    Ok(size) => Some(buf[..size].to_vec()),
                }
            })
        };
        assert_eq!(output, b"test\n");

        match content.unwrap() {
            Content::StdinWritten(_) => (),
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_write_stdin_should_send_error_if_process_exited() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;

        let id = 999;
        let input = b"test\n".to_vec();
        let child = Command::new("echo")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        state.procs.insert(id, LocalProc::try_from(child).unwrap());

        // Give process some time to run and complete
        thread::sleep(Duration::from_millis(10));

        do_write_stdin(&mut state, &DoWriteStdinArgs { id, input }, |c| {
            content = Some(c);
            Ok(())
        })
        .await
        .unwrap();

        match content.unwrap() {
            Content::IoError(IoErrorArgs { error_kind, .. }) => {
                assert_eq!(error_kind, io::ErrorKind::BrokenPipe);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_write_stdin_should_send_error_if_process_id_not_registered() {
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
        .await
        .unwrap();

        match content.unwrap() {
            Content::IoError(IoErrorArgs { error_kind, .. }) => {
                assert_eq!(error_kind, io::ErrorKind::InvalidInput);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_get_stdout_should_send_contents_if_process_sent_stdout() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;

        let id = 999;
        let child = Command::new("echo")
            .arg("test")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        state.procs.insert(id, LocalProc::try_from(child).unwrap());

        // Give process some time to run and complete
        thread::sleep(Duration::from_millis(10));

        do_get_stdout(&mut state, &DoGetStdoutArgs { id }, |c| {
            content = Some(c);
            Ok(())
        })
        .await
        .unwrap();

        match content.unwrap() {
            Content::StdoutContents(StdoutContentsArgs { output }) => {
                assert_eq!(output, b"test\n");
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_get_stdout_should_send_empty_contents_if_process_has_no_stdout() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;

        let id = 999;
        let child = Command::new("cat")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        state.procs.insert(id, LocalProc::try_from(child).unwrap());

        // Give process some time to start
        thread::sleep(Duration::from_millis(10));

        do_get_stdout(&mut state, &DoGetStdoutArgs { id }, |c| {
            content = Some(c);
            Ok(())
        })
        .await
        .unwrap();

        match content.unwrap() {
            Content::StdoutContents(StdoutContentsArgs { output }) => {
                assert!(output.is_empty());
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_get_stdout_should_send_error_if_process_id_not_registered() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;

        do_get_stdout(&mut state, &DoGetStdoutArgs { id: 0 }, |c| {
            content = Some(c);
            Ok(())
        })
        .await
        .unwrap();

        match content.unwrap() {
            Content::IoError(IoErrorArgs { error_kind, .. }) => {
                assert_eq!(error_kind, io::ErrorKind::InvalidInput);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_get_stderr_should_send_contents_if_process_sent_stderr() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;

        let id = 999;
        let child = Command::new("rev")
            .arg("--aaa")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        state.procs.insert(id, LocalProc::try_from(child).unwrap());

        // Give process some time to run and complete
        thread::sleep(Duration::from_millis(10));

        do_get_stderr(&mut state, &DoGetStderrArgs { id }, |c| {
            content = Some(c);
            Ok(())
        })
        .await
        .unwrap();

        match content.unwrap() {
            Content::StderrContents(StderrContentsArgs { output }) => {
                assert!(output.len() > 0);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_get_stderr_should_send_empty_contents_if_process_has_no_stderr() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;

        let id = 999;
        let child = Command::new("cat")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        state.procs.insert(id, LocalProc::try_from(child).unwrap());

        // Give process some time to start
        thread::sleep(Duration::from_millis(10));

        do_get_stderr(&mut state, &DoGetStderrArgs { id }, |c| {
            content = Some(c);
            Ok(())
        })
        .await
        .unwrap();

        match content.unwrap() {
            Content::StderrContents(StderrContentsArgs { output }) => {
                assert!(output.is_empty());
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_get_stderr_should_send_error_if_process_id_not_registered() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;

        do_get_stderr(&mut state, &DoGetStderrArgs { id: 0 }, |c| {
            content = Some(c);
            Ok(())
        })
        .await
        .unwrap();

        match content.unwrap() {
            Content::IoError(IoErrorArgs { error_kind, .. }) => {
                assert_eq!(error_kind, io::ErrorKind::InvalidInput);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_proc_kill_should_send_exit_status_after_killing_process() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;

        let id = 999;
        let child = Command::new("sleep")
            .arg("10")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        state.procs.insert(id, LocalProc::try_from(child).unwrap());

        // Give process some time to start
        thread::sleep(Duration::from_millis(10));

        do_kill_proc(&mut state, &DoKillProcArgs { id }, |c| {
            content = Some(c);
            Ok(())
        })
        .await
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

    #[tokio::test]
    async fn do_proc_kill_should_send_exit_status_if_process_already_exited() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;

        let id = 999;
        let child = Command::new("echo")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        state.procs.insert(id, LocalProc::try_from(child).unwrap());

        // Give process some time to run and complete
        thread::sleep(Duration::from_millis(10));

        do_kill_proc(&mut state, &DoKillProcArgs { id }, |c| {
            content = Some(c);
            Ok(())
        })
        .await
        .unwrap();

        match content.unwrap() {
            Content::ProcStatus(ProcStatusArgs { exit_code, .. }) => assert_eq!(exit_code, Some(0)),
            x => panic!("Bad content: {:?}", x),
        }
    }
}
