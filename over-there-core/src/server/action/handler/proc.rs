use crate::{
    msg::content::*,
    server::{action::ActionError, proc::LocalProc, state::ServerState},
};
use log::debug;
use std::future::Future;
use std::io;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;

pub async fn do_exec_proc<F, R>(
    state: Arc<ServerState>,
    args: &DoExecProcArgs,
    respond: F,
) -> Result<(), ActionError>
where
    F: FnOnce(Content) -> R,
    R: Future<Output = Result<(), ActionError>>,
{
    debug!("do_exec_proc: {:?}", args);
    let DoExecProcArgs {
        command,
        args,
        stdin,
        stdout,
        stderr,
        current_dir,
    } = args;

    let make_pipe = |yes| if yes { Stdio::piped() } else { Stdio::null() };

    let mut cmd = Command::new(command);
    cmd.args(args)
        .stdin(make_pipe(*stdin))
        .stdout(make_pipe(*stdout))
        .stderr(make_pipe(*stderr))
        .kill_on_drop(true);

    // If provided a directory to change to, set that with the command
    if let Some(dir) = current_dir {
        // NOTE: It is recommended to canonicalize the path before applying
        //       it to ensure that it is absolute as platforms can apply
        //       relative or absolute differently otherwise
        match tokio::fs::canonicalize(dir).await {
            Ok(dir) => {
                cmd.current_dir(dir);
            }
            Err(x) => return respond(Content::IoError(From::from(x))).await,
        }
    }

    match cmd.spawn() {
        Ok(child) => {
            let local_proc = LocalProc::new(child).spawn();
            let id = local_proc.id();
            state.procs.lock().await.insert(id, local_proc);
            state.touch_proc_id(id).await;
            respond(Content::ProcStarted(ProcStartedArgs { id })).await
        }
        Err(x) => respond(Content::IoError(From::from(x))).await,
    }
}

pub async fn do_write_stdin<F, R>(
    state: Arc<ServerState>,
    args: &DoWriteStdinArgs,
    respond: F,
) -> Result<(), ActionError>
where
    F: FnOnce(Content) -> R,
    R: Future<Output = Result<(), ActionError>>,
{
    debug!("do_write_stdin: {:?}", args);
    state.touch_proc_id(args.id).await;

    match state.procs.lock().await.get_mut(&args.id) {
        Some(local_proc) => match local_proc.write_stdin(&args.input).await {
            Ok(_) => {
                respond(Content::StdinWritten(StdinWrittenArgs { id: args.id }))
                    .await
            }
            Err(x) => respond(Content::IoError(From::from(x))).await,
        },
        None => {
            respond(Content::IoError(IoErrorArgs::invalid_proc_id(args.id)))
                .await
        }
    }
}

pub async fn do_get_stdout<F, R>(
    state: Arc<ServerState>,
    args: &DoGetStdoutArgs,
    respond: F,
) -> Result<(), ActionError>
where
    F: FnOnce(Content) -> R,
    R: Future<Output = Result<(), ActionError>>,
{
    debug!("do_get_stdout: {:?}", args);
    state.touch_proc_id(args.id).await;

    match state.procs.lock().await.get_mut(&args.id) {
        Some(local_proc) => match local_proc.read_stdout().await {
            Ok(output) => {
                respond(Content::StdoutContents(StdoutContentsArgs {
                    id: args.id,
                    output,
                }))
                .await
            }
            Err(x) if x.kind() == io::ErrorKind::WouldBlock => {
                respond(Content::StdoutContents(StdoutContentsArgs {
                    id: args.id,
                    output: vec![],
                }))
                .await
            }
            Err(x) => respond(Content::IoError(From::from(x))).await,
        },
        None => {
            respond(Content::IoError(IoErrorArgs::invalid_proc_id(args.id)))
                .await
        }
    }
}

pub async fn do_get_stderr<F, R>(
    state: Arc<ServerState>,
    args: &DoGetStderrArgs,
    respond: F,
) -> Result<(), ActionError>
where
    F: FnOnce(Content) -> R,
    R: Future<Output = Result<(), ActionError>>,
{
    debug!("do_get_stderr: {:?}", args);
    state.touch_proc_id(args.id).await;

    match state.procs.lock().await.get_mut(&args.id) {
        Some(local_proc) => match local_proc.read_stderr().await {
            Ok(output) => {
                respond(Content::StderrContents(StderrContentsArgs {
                    id: args.id,
                    output,
                }))
                .await
            }
            Err(x) if x.kind() == io::ErrorKind::WouldBlock => {
                respond(Content::StderrContents(StderrContentsArgs {
                    id: args.id,
                    output: vec![],
                }))
                .await
            }
            Err(x) => respond(Content::IoError(From::from(x))).await,
        },
        None => {
            respond(Content::IoError(IoErrorArgs::invalid_proc_id(args.id)))
                .await
        }
    }
}

pub async fn do_get_proc_status<F, R>(
    state: Arc<ServerState>,
    args: &DoGetProcStatusArgs,
    respond: F,
) -> Result<(), ActionError>
where
    F: FnOnce(Content) -> R,
    R: Future<Output = Result<(), ActionError>>,
{
    debug!("do_get_proc_status: {:?}", args);
    state.touch_proc_id(args.id).await;

    match state.procs.lock().await.get_mut(&args.id) {
        Some(local_proc) => match local_proc.exit_status().await {
            Some(exit_status) => {
                // Process is now dead, so we want to touch with a smaller
                // cleanup TTL while still allowing this to be polled
                state
                    .touch_proc_id_with_ttl(args.id, state.dead_proc_ttl)
                    .await;

                respond(Content::ProcStatus(ProcStatusArgs {
                    id: args.id,
                    is_alive: false,
                    exit_code: exit_status.exit_code,
                }))
                .await
            }
            None => {
                respond(Content::ProcStatus(ProcStatusArgs {
                    id: args.id,
                    is_alive: true,
                    exit_code: None,
                }))
                .await
            }
        },
        None => {
            respond(Content::IoError(IoErrorArgs::invalid_proc_id(args.id)))
                .await
        }
    }
}

pub async fn do_kill_proc<F, R>(
    state: Arc<ServerState>,
    args: &DoKillProcArgs,
    respond: F,
) -> Result<(), ActionError>
where
    F: FnOnce(Content) -> R,
    R: Future<Output = Result<(), ActionError>>,
{
    debug!("do_kill_proc: {:?}", args);
    state.touch_proc_id(args.id).await;

    match state.procs.lock().await.remove(&args.id) {
        // NOTE: We are killing and then WAITING for the process to die, which
        //       would block, but seems to be required in order to properly
        //       have the process clean up -- try_wait doesn't seem to work
        Some(local_proc) => match local_proc.kill_and_wait().await {
            Ok(output) => {
                state.remove_proc_id(args.id).await;

                // TODO: Send stdout/stderr msgs for any remaining content
                respond(Content::ProcStatus(ProcStatusArgs {
                    id: args.id,
                    is_alive: false,
                    exit_code: output.status.code(),
                }))
                .await
            }
            Err(x) => respond(Content::IoError(From::from(x))).await,
        },
        None => {
            respond(Content::IoError(IoErrorArgs::invalid_proc_id(args.id)))
                .await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::process::Stdio;
    use std::time::Duration;
    use tokio::{
        task,
        time::{delay_for, timeout},
    };

    #[tokio::test]
    async fn do_exec_proc_should_send_success_if_can_execute_process() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        do_exec_proc(
            Arc::clone(&state),
            &DoExecProcArgs {
                command: String::from("rev"),
                args: vec![String::from("test")],
                stdin: false,
                stdout: false,
                stderr: false,
                current_dir: None,
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        match content.unwrap() {
            Content::ProcStarted(args) => {
                let x = state.procs.lock().await;
                let proc = x.get(&args.id).unwrap();
                assert_eq!(proc.id(), args.id);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_exec_proc_should_set_current_dir_if_provided() {
        let tempdir = tempfile::tempdir().expect("Failed to create temp dir");
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        do_exec_proc(
            Arc::clone(&state),
            &DoExecProcArgs {
                command: String::from("touch"),
                args: vec![String::from("test-file")],
                stdin: false,
                stdout: false,
                stderr: false,
                current_dir: Some(
                    tempdir.as_ref().to_string_lossy().to_string(),
                ),
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        match content.unwrap() {
            Content::ProcStarted(_) => (),
            x => panic!("Bad content: {:?}", x),
        }

        // Give above some time to fully execute
        tokio::time::delay_for(Duration::from_millis(50)).await;

        let path = tempdir.as_ref().join("test-file");
        assert!(path.exists());
    }

    #[tokio::test]
    async fn do_exec_proc_should_send_error_if_process_does_not_exist() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        do_exec_proc(
            Arc::clone(&state),
            &DoExecProcArgs {
                command: String::from("<a><b><c>"),
                args: vec![],
                stdin: false,
                stdout: false,
                stderr: false,
                current_dir: None,
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        match content.unwrap() {
            Content::IoError(args) => {
                assert_eq!(args.error_kind, io::ErrorKind::NotFound)
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_write_stdin_should_send_data_to_running_process() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let id = 999;
        let input = b"test\n".to_vec();
        let child = Command::new("cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        state
            .procs
            .lock()
            .await
            .insert(id, LocalProc::new(child).spawn());

        // Give process some time to start
        delay_for(Duration::from_millis(10)).await;

        do_write_stdin(
            Arc::clone(&state),
            &DoWriteStdinArgs { id, input },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        let output = {
            let mut x = state.procs.lock().await;
            let local_proc = x.get_mut(&id).unwrap();

            match timeout(Duration::from_millis(500), async {
                loop {
                    match local_proc.read_stdout().await {
                        Ok(buf) if buf.is_empty() => task::yield_now().await,
                        x => break x,
                    }
                }
            })
            .await
            {
                Ok(result) if result.is_ok() => result.unwrap(),
                Ok(result) => {
                    panic!("Unexpected error {}", result.unwrap_err())
                }
                Err(x) => panic!("Timeout {}", x),
            }
        };
        assert_eq!(output, b"test\n");

        match content.unwrap() {
            Content::StdinWritten(_) => (),
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_write_stdin_should_send_error_if_process_exited() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let id = 999;
        let input = b"test\n".to_vec();
        let child = Command::new("echo")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        state
            .procs
            .lock()
            .await
            .insert(id, LocalProc::new(child).spawn());

        // Give process some time to run and complete
        delay_for(Duration::from_millis(10)).await;

        do_write_stdin(
            Arc::clone(&state),
            &DoWriteStdinArgs { id, input },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
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
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        do_write_stdin(
            Arc::clone(&state),
            &DoWriteStdinArgs {
                id: 0,
                input: b"test\n".to_vec(),
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
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
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let id = 999;
        let child = Command::new("echo")
            .arg("test")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        state
            .procs
            .lock()
            .await
            .insert(id, LocalProc::new(child).spawn());

        // Give process some time to run and complete
        delay_for(Duration::from_millis(10)).await;

        do_get_stdout(Arc::clone(&state), &DoGetStdoutArgs { id }, |c| {
            content = Some(c);
            async { Ok(()) }
        })
        .await
        .unwrap();

        match content.unwrap() {
            Content::StdoutContents(StdoutContentsArgs {
                id: arg_id,
                output,
            }) => {
                assert_eq!(arg_id, id, "Wrong id returned");
                assert_eq!(output, b"test\n");
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_get_stdout_should_send_empty_contents_if_process_has_no_stdout()
    {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let id = 999;
        let child = Command::new("cat")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        state
            .procs
            .lock()
            .await
            .insert(id, LocalProc::new(child).spawn());

        // Give process some time to start
        delay_for(Duration::from_millis(10)).await;

        do_get_stdout(Arc::clone(&state), &DoGetStdoutArgs { id }, |c| {
            content = Some(c);
            async { Ok(()) }
        })
        .await
        .unwrap();

        match content.unwrap() {
            Content::StdoutContents(StdoutContentsArgs {
                id: arg_id,
                output,
            }) => {
                assert_eq!(arg_id, id, "Wrong id returned");
                assert!(output.is_empty());
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_get_stdout_should_send_error_if_process_id_not_registered() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        do_get_stdout(Arc::clone(&state), &DoGetStdoutArgs { id: 0 }, |c| {
            content = Some(c);
            async { Ok(()) }
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
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let id = 999;
        let child = Command::new("rev")
            .arg("--aaa")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        state
            .procs
            .lock()
            .await
            .insert(id, LocalProc::new(child).spawn());

        // Give process some time to run and complete
        delay_for(Duration::from_millis(10)).await;

        do_get_stderr(Arc::clone(&state), &DoGetStderrArgs { id }, |c| {
            content = Some(c);
            async { Ok(()) }
        })
        .await
        .unwrap();

        match content.unwrap() {
            Content::StderrContents(StderrContentsArgs {
                id: arg_id,
                output,
            }) => {
                assert_eq!(arg_id, id, "Wrong id returned");
                assert!(output.len() > 0);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_get_stderr_should_send_empty_contents_if_process_has_no_stderr()
    {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let id = 999;
        let child = Command::new("cat")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        state
            .procs
            .lock()
            .await
            .insert(id, LocalProc::new(child).spawn());

        // Give process some time to start
        delay_for(Duration::from_millis(10)).await;

        do_get_stderr(Arc::clone(&state), &DoGetStderrArgs { id }, |c| {
            content = Some(c);
            async { Ok(()) }
        })
        .await
        .unwrap();

        match content.unwrap() {
            Content::StderrContents(StderrContentsArgs {
                id: arg_id,
                output,
            }) => {
                assert_eq!(arg_id, id, "Wrong id returned");
                assert!(output.is_empty());
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_get_stderr_should_send_error_if_process_id_not_registered() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        do_get_stderr(Arc::clone(&state), &DoGetStderrArgs { id: 0 }, |c| {
            content = Some(c);
            async { Ok(()) }
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
    async fn do_get_proc_status_should_send_status_if_process_still_alive() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let id = 999;
        let child = Command::new("sleep")
            .arg("10")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        state
            .procs
            .lock()
            .await
            .insert(id, LocalProc::new(child).spawn());

        // Give process some time to start
        delay_for(Duration::from_millis(10)).await;

        do_get_proc_status(
            Arc::clone(&state),
            &DoGetProcStatusArgs { id },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        match content.unwrap() {
            Content::ProcStatus(ProcStatusArgs {
                id: status_id,
                is_alive,
                exit_code,
            }) => {
                assert_eq!(status_id, id);
                assert!(is_alive);
                assert_eq!(exit_code, None);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_get_proc_status_should_send_exit_status_if_process_exited() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let id = 999;
        let child = Command::new("echo")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        state
            .procs
            .lock()
            .await
            .insert(id, LocalProc::new(child).spawn());

        // Give process some time to start
        delay_for(Duration::from_millis(10)).await;

        do_get_proc_status(
            Arc::clone(&state),
            &DoGetProcStatusArgs { id },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        match content.unwrap() {
            Content::ProcStatus(ProcStatusArgs {
                id: exit_id,
                is_alive,
                exit_code,
            }) => {
                assert_eq!(exit_id, id);
                assert!(!is_alive);
                assert_eq!(exit_code, Some(0));
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_proc_kill_should_send_exit_status_after_killing_process() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let id = 999;
        let child = Command::new("sleep")
            .arg("10")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        state
            .procs
            .lock()
            .await
            .insert(id, LocalProc::new(child).spawn());

        // Give process some time to start
        delay_for(Duration::from_millis(10)).await;

        do_kill_proc(Arc::clone(&state), &DoKillProcArgs { id }, |c| {
            content = Some(c);
            async { Ok(()) }
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
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let id = 999;
        let child = Command::new("echo")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        state
            .procs
            .lock()
            .await
            .insert(id, LocalProc::new(child).spawn());

        // Give process some time to run and complete
        delay_for(Duration::from_millis(10)).await;

        do_kill_proc(Arc::clone(&state), &DoKillProcArgs { id }, |c| {
            content = Some(c);
            async { Ok(()) }
        })
        .await
        .unwrap();

        match content.unwrap() {
            Content::ProcStatus(ProcStatusArgs { exit_code, .. }) => {
                assert_eq!(exit_code, Some(0))
            }
            x => panic!("Bad content: {:?}", x),
        }
    }
}
