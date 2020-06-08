use crate::core::{
    reply::*,
    request::*,
    server::{proc::LocalProc, state::ServerState},
};
use log::debug;
use std::io;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;

pub async fn exec_proc(
    state: Arc<ServerState>,
    args: &ExecProcArgs,
) -> Result<ProcStartedArgs, io::Error> {
    debug!("handler::exec_proc: {:?}", args);
    let ExecProcArgs {
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
        let dir = tokio::fs::canonicalize(dir).await?;
        cmd.current_dir(dir);
    }

    let child = cmd.spawn()?;
    let local_proc = LocalProc::new(child).spawn();
    let id = local_proc.id();
    state.procs.lock().await.insert(id, local_proc);
    state.touch_proc_id(id).await;
    Ok(ProcStartedArgs { id })
}

pub async fn write_proc_stdin(
    state: Arc<ServerState>,
    args: &WriteProcStdinArgs,
) -> Result<ProcStdinWrittenArgs, io::Error> {
    debug!("handler::write_proc_stdin: {:?}", args);
    state.touch_proc_id(args.id).await;

    match state.procs.lock().await.get_mut(&args.id) {
        Some(local_proc) => {
            local_proc.write_stdin(&args.input).await?;
            Ok(ProcStdinWrittenArgs { id: args.id })
        }
        None => Err(IoErrorArgs::invalid_proc_id(args.id).into()),
    }
}

pub async fn read_proc_stdout(
    state: Arc<ServerState>,
    args: &ReadProcStdoutArgs,
) -> Result<ProcStdoutContentsArgs, io::Error> {
    debug!("handler::read_proc_stdout: {:?}", args);
    state.touch_proc_id(args.id).await;

    match state.procs.lock().await.get_mut(&args.id) {
        Some(local_proc) => match local_proc.read_stdout().await {
            Ok(output) => Ok(ProcStdoutContentsArgs {
                id: args.id,
                output,
            }),
            Err(x) if x.kind() == io::ErrorKind::WouldBlock => {
                Ok(ProcStdoutContentsArgs {
                    id: args.id,
                    output: vec![],
                })
            }
            Err(x) => Err(x),
        },
        None => Err(IoErrorArgs::invalid_proc_id(args.id).into()),
    }
}

pub async fn read_proc_stderr(
    state: Arc<ServerState>,
    args: &ReadProcStderrArgs,
) -> Result<ProcStderrContentsArgs, io::Error> {
    debug!("handler::read_proc_stderr: {:?}", args);
    state.touch_proc_id(args.id).await;

    match state.procs.lock().await.get_mut(&args.id) {
        Some(local_proc) => match local_proc.read_stderr().await {
            Ok(output) => Ok(ProcStderrContentsArgs {
                id: args.id,
                output,
            }),
            Err(x) if x.kind() == io::ErrorKind::WouldBlock => {
                Ok(ProcStderrContentsArgs {
                    id: args.id,
                    output: vec![],
                })
            }
            Err(x) => Err(x),
        },
        None => Err(IoErrorArgs::invalid_proc_id(args.id).into()),
    }
}

pub async fn read_proc_status(
    state: Arc<ServerState>,
    args: &ReadProcStatusArgs,
) -> Result<ProcStatusArgs, io::Error> {
    debug!("handler::read_proc_status: {:?}", args);
    state.touch_proc_id(args.id).await;

    match state.procs.lock().await.get_mut(&args.id) {
        Some(local_proc) => match local_proc.exit_status().await {
            Some(exit_status) => {
                // Process is now dead, so we want to touch with a smaller
                // cleanup TTL while still allowing this to be polled
                state
                    .touch_proc_id_with_ttl(args.id, state.dead_proc_ttl)
                    .await;

                Ok(ProcStatusArgs {
                    id: args.id,
                    is_alive: false,
                    exit_code: exit_status.exit_code,
                })
            }
            None => Ok(ProcStatusArgs {
                id: args.id,
                is_alive: true,
                exit_code: None,
            }),
        },
        None => Err(IoErrorArgs::invalid_proc_id(args.id).into()),
    }
}

pub async fn kill_proc(
    state: Arc<ServerState>,
    args: &KillProcArgs,
) -> Result<ProcKilledArgs, io::Error> {
    debug!("handler::kill_proc: {:?}", args);
    state.touch_proc_id(args.id).await;

    match state.procs.lock().await.remove(&args.id) {
        // NOTE: We are killing and then WAITING for the process to die, which
        //       would block, but seems to be required in order to properly
        //       have the process clean up -- try_wait doesn't seem to work
        Some(local_proc) => {
            let output = local_proc.kill_and_wait().await?;
            state.remove_proc_id(args.id).await;

            // TODO: Send stdout/stderr msgs for any remaining content
            Ok(ProcKilledArgs {
                id: args.id,
                exit_code: output.status.code(),
            })
        }
        None => Err(IoErrorArgs::invalid_proc_id(args.id).into()),
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
    async fn exec_proc_should_return_success_if_can_execute_process() {
        let state = Arc::new(ServerState::default());

        let args = exec_proc(
            Arc::clone(&state),
            &ExecProcArgs {
                command: String::from("rev"),
                args: vec![String::from("test")],
                stdin: false,
                stdout: false,
                stderr: false,
                current_dir: None,
            },
        )
        .await
        .unwrap();

        let x = state.procs.lock().await;
        let proc = x.get(&args.id).unwrap();
        assert_eq!(proc.id(), args.id);
    }

    #[tokio::test]
    async fn exec_proc_should_set_current_dir_if_provided() {
        let tempdir = tempfile::tempdir().expect("Failed to create temp dir");
        let state = Arc::new(ServerState::default());

        let _ = exec_proc(
            Arc::clone(&state),
            &ExecProcArgs {
                command: String::from("touch"),
                args: vec![String::from("test-file")],
                stdin: false,
                stdout: false,
                stderr: false,
                current_dir: Some(
                    tempdir.as_ref().to_string_lossy().to_string(),
                ),
            },
        )
        .await
        .unwrap();

        // Give above some time to fully execute
        tokio::time::delay_for(Duration::from_millis(50)).await;

        let path = tempdir.as_ref().join("test-file");
        assert!(path.exists());
    }

    #[tokio::test]
    async fn exec_proc_should_return_error_if_process_does_not_exist() {
        let state = Arc::new(ServerState::default());

        let err = exec_proc(
            Arc::clone(&state),
            &ExecProcArgs {
                command: String::from("<a><b><c>"),
                args: vec![],
                stdin: false,
                stdout: false,
                stderr: false,
                current_dir: None,
            },
        )
        .await
        .unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[tokio::test]
    async fn write_proc_stdin_should_return_data_to_running_process() {
        let state = Arc::new(ServerState::default());

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

        let args = write_proc_stdin(
            Arc::clone(&state),
            &WriteProcStdinArgs { id, input },
        )
        .await
        .unwrap();
        assert_eq!(args.id, id);

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
    }

    #[tokio::test]
    async fn write_proc_stdin_should_return_error_if_process_exited() {
        let state = Arc::new(ServerState::default());

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

        let err = write_proc_stdin(
            Arc::clone(&state),
            &WriteProcStdinArgs { id, input },
        )
        .await
        .unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::BrokenPipe);
    }

    #[tokio::test]
    async fn write_proc_stdin_should_return_error_if_process_id_not_registered()
    {
        let state = Arc::new(ServerState::default());

        let err = write_proc_stdin(
            Arc::clone(&state),
            &WriteProcStdinArgs {
                id: 0,
                input: b"test\n".to_vec(),
            },
        )
        .await
        .unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[tokio::test]
    async fn read_proc_stdout_should_return_contents_if_process_sent_stdout() {
        let state = Arc::new(ServerState::default());

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

        let args =
            read_proc_stdout(Arc::clone(&state), &ReadProcStdoutArgs { id })
                .await
                .unwrap();

        assert_eq!(args.id, id, "Wrong id returned");
        assert_eq!(args.output, b"test\n");
    }

    #[tokio::test]
    async fn read_proc_stdout_should_return_empty_contents_if_process_has_no_stdout(
    ) {
        let state = Arc::new(ServerState::default());

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

        let args =
            read_proc_stdout(Arc::clone(&state), &ReadProcStdoutArgs { id })
                .await
                .unwrap();

        assert_eq!(args.id, id, "Wrong id returned");
        assert!(args.output.is_empty());
    }

    #[tokio::test]
    async fn read_proc_stdout_should_return_error_if_process_id_not_registered()
    {
        let state = Arc::new(ServerState::default());

        let err =
            read_proc_stdout(Arc::clone(&state), &ReadProcStdoutArgs { id: 0 })
                .await
                .unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[tokio::test]
    async fn read_proc_stderr_should_return_contents_if_process_sent_stderr() {
        let state = Arc::new(ServerState::default());

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

        let args =
            read_proc_stderr(Arc::clone(&state), &ReadProcStderrArgs { id })
                .await
                .unwrap();

        assert_eq!(args.id, id, "Wrong id returned");
        assert!(args.output.len() > 0);
    }

    #[tokio::test]
    async fn read_proc_stderr_should_return_empty_contents_if_process_has_no_stderr(
    ) {
        let state = Arc::new(ServerState::default());

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

        let args =
            read_proc_stderr(Arc::clone(&state), &ReadProcStderrArgs { id })
                .await
                .unwrap();

        assert_eq!(args.id, id, "Wrong id returned");
        assert!(args.output.is_empty());
    }

    #[tokio::test]
    async fn read_proc_stderr_should_return_error_if_process_id_not_registered()
    {
        let state = Arc::new(ServerState::default());

        let err =
            read_proc_stderr(Arc::clone(&state), &ReadProcStderrArgs { id: 0 })
                .await
                .unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[tokio::test]
    async fn read_proc_status_should_return_status_if_process_still_alive() {
        let state = Arc::new(ServerState::default());

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

        let args =
            read_proc_status(Arc::clone(&state), &ReadProcStatusArgs { id })
                .await
                .unwrap();

        assert_eq!(args.id, id);
        assert!(args.is_alive);
        assert_eq!(args.exit_code, None);
    }

    #[tokio::test]
    async fn read_proc_status_should_return_exit_status_if_process_exited() {
        let state = Arc::new(ServerState::default());

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

        let args =
            read_proc_status(Arc::clone(&state), &ReadProcStatusArgs { id })
                .await
                .unwrap();

        assert_eq!(args.id, id);
        assert!(!args.is_alive);
        assert_eq!(args.exit_code, Some(0));
    }

    #[tokio::test]
    async fn proc_kill_should_return_exit_status_after_killing_process() {
        let state = Arc::new(ServerState::default());

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

        let args = kill_proc(Arc::clone(&state), &KillProcArgs { id })
            .await
            .unwrap();

        assert_eq!(args.id, id);
    }

    #[tokio::test]
    async fn proc_kill_should_return_exit_status_if_process_already_exited() {
        let state = Arc::new(ServerState::default());

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

        let args = kill_proc(Arc::clone(&state), &KillProcArgs { id })
            .await
            .unwrap();

        assert_eq!(args.exit_code, Some(0))
    }
}
