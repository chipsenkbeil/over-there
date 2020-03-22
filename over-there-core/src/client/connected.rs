use super::{
    error::{AskError, ExecAskError, FileAskError, TellError},
    file::RemoteFile,
    proc::{RemoteProc, RemoteProcStatus},
    state::ClientState,
};
use crate::{
    event::{AddrEventManager, EventManager},
    msg::{
        content::{
            capabilities::Capability,
            internal_debug::InternalDebugArgs,
            io::{fs::*, proc::*},
            Content,
        },
        Msg,
    },
};
use log::{error, trace};
use over_there_utils::Either;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::{
    sync::{oneshot, Mutex},
    task::{JoinError, JoinHandle},
};

/// Represents a client after connecting to an endpoint
pub struct ConnectedClient {
    pub(super) state: Arc<Mutex<ClientState>>,

    /// Represents the event manager used to send and receive data
    pub(super) event_manager: Either<EventManager, AddrEventManager>,

    /// Represents the handle for processing events
    pub(super) event_handle: JoinHandle<()>,

    /// Represents the address the client is connected to
    pub(super) remote_addr: SocketAddr,

    /// Represents maximum to wait on responses before timing out
    pub timeout: Duration,
}

impl ConnectedClient {
    /// Default timeout applied to a new client for any ask made
    pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

    pub fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }

    pub async fn wait(self) -> Result<(), JoinError> {
        match self.event_manager {
            Either::Left(m) => {
                tokio::try_join!(m.wait(), self.event_handle).map(|_| ())
            }
            Either::Right(m) => {
                tokio::try_join!(m.wait(), self.event_handle).map(|_| ())
            }
        }
    }

    /// Generic ask of the server that is expecting a response
    pub async fn ask(&mut self, msg: Msg) -> Result<Msg, AskError> {
        let timeout = self.timeout;
        let (tx, rx) = oneshot::channel::<Result<Msg, AskError>>();

        // Assign a synchronous callback that uses the oneshot channel to
        // get back the result
        self.state.lock().await.callback_manager.add_callback(
            msg.header.id,
            |msg| {
                let result = if let Content::Error(args) = &msg.content {
                    tx.send(Err(AskError::Failure {
                        msg: args.msg.to_string(),
                    }))
                } else {
                    tx.send(Ok(msg.clone()))
                };

                if result.is_err() {
                    error!("Failed to trigger callback: {:?}", msg);
                }
            },
        );

        // Send the msg and report back an error if it occurs
        self.tell(msg).await.map_err(AskError::from)?;

        tokio::time::timeout(timeout, rx)
            .await
            .map_err(|_| AskError::Timeout)?
            .map_err(|_| AskError::CallbackLost)?
    }

    /// Sends a msg to the server, not expecting a response
    pub async fn tell(&mut self, msg: Msg) -> Result<(), TellError> {
        trace!("Sending to {}: {:?}", self.remote_addr, msg);

        let data = msg.to_vec().map_err(|_| TellError::EncodingFailed)?;
        match &mut self.event_manager {
            Either::Left(m) => {
                m.send(data).await.map_err(|_| TellError::SendFailed)
            }
            Either::Right(m) => m
                .send_to(data, self.remote_addr)
                .await
                .map_err(|_| TellError::SendFailed),
        }
    }

    /// Requests the version from the server
    pub async fn ask_version(&mut self) -> Result<String, AskError> {
        let msg = self.ask(Msg::from(Content::DoGetVersion)).await?;
        match msg.content {
            Content::Version(args) => Ok(args.version),
            x => Err(make_ask_error(x)),
        }
    }

    /// Requests the capabilities from the server
    pub async fn ask_capabilities(
        &mut self,
    ) -> Result<Vec<Capability>, AskError> {
        let msg = self.ask(Msg::from(Content::DoGetCapabilities)).await?;
        match msg.content {
            Content::Capabilities(args) => Ok(args.capabilities),
            x => Err(make_ask_error(x)),
        }
    }

    /// Requests to create a new directory
    pub async fn ask_create_dir(
        &mut self,
        path: String,
        include_components: bool,
    ) -> Result<(), FileAskError> {
        let result = self
            .ask(Msg::from(Content::DoCreateDir(DoCreateDirArgs {
                path,
                include_components,
            })))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap().content {
            Content::DirCreated(_) => Ok(()),
            x => Err(make_file_ask_error(x)),
        }
    }

    /// Requests to rename an existing directory
    pub async fn ask_rename_dir(
        &mut self,
        from: String,
        to: String,
    ) -> Result<(), FileAskError> {
        let result = self
            .ask(Msg::from(Content::DoRenameDir(DoRenameDirArgs {
                from,
                to,
            })))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap().content {
            Content::DirRenamed(_) => Ok(()),
            x => Err(make_file_ask_error(x)),
        }
    }

    /// Requests to remove an existing directory
    pub async fn ask_remove_dir(
        &mut self,
        path: String,
        non_empty: bool,
    ) -> Result<(), FileAskError> {
        let result = self
            .ask(Msg::from(Content::DoRemoveDir(DoRemoveDirArgs {
                path,
                non_empty,
            })))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap().content {
            Content::DirRemoved(_) => Ok(()),
            x => Err(make_file_ask_error(x)),
        }
    }

    /// Requests to get a list of the root directory's contents on the server
    pub async fn ask_list_root_dir_contents(
        &mut self,
    ) -> Result<Vec<DirEntry>, FileAskError> {
        self.ask_list_dir_contents(String::from(".")).await
    }

    /// Requests to get a list of a directory's contents on the server
    pub async fn ask_list_dir_contents(
        &mut self,
        path: String,
    ) -> Result<Vec<DirEntry>, FileAskError> {
        let result = self
            .ask(Msg::from(Content::DoListDirContents(
                DoListDirContentsArgs { path },
            )))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap().content {
            Content::DirContentsList(args) => Ok(args.entries),
            x => Err(make_file_ask_error(x)),
        }
    }

    /// Requests to open a file for reading/writing on the server,
    /// creating the file if it does not exist
    pub async fn ask_open_file(
        &mut self,
        path: String,
    ) -> Result<RemoteFile, FileAskError> {
        self.ask_open_file_with_options(path, true, true, true)
            .await
    }

    /// Requests to open a file on the server, opening using the provided options
    pub async fn ask_open_file_with_options(
        &mut self,
        path: String,
        create: bool,
        write: bool,
        read: bool,
    ) -> Result<RemoteFile, FileAskError> {
        let result = self
            .ask(Msg::from(Content::DoOpenFile(DoOpenFileArgs {
                path: path.clone(),
                create_if_missing: create,
                write_access: write,
                read_access: read,
            })))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap().content {
            Content::FileOpened(args) => Ok(RemoteFile {
                id: args.id,
                sig: args.sig,
                path,
            }),
            x => Err(make_file_ask_error(x)),
        }
    }

    /// Requests to close an open file
    pub async fn ask_close_file(
        &mut self,
        file: &RemoteFile,
    ) -> Result<(), FileAskError> {
        let result = self
            .ask(Msg::from(Content::DoCloseFile(DoCloseFileArgs {
                id: file.id,
                sig: file.sig,
            })))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap().content {
            Content::FileClosed(_) => Ok(()),
            x => Err(make_file_ask_error(x)),
        }
    }

    /// Requests to rename an open file
    pub async fn ask_rename_file(
        &mut self,
        file: &mut RemoteFile,
        to: String,
    ) -> Result<(), FileAskError> {
        let result = self
            .ask(Msg::from(Content::DoRenameFile(DoRenameFileArgs {
                id: file.id,
                sig: file.sig,
                to,
            })))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap().content {
            Content::FileRenamed(FileRenamedArgs { sig }) => {
                file.sig = sig;
                Ok(())
            }
            x => Err(make_file_ask_error(x)),
        }
    }

    /// Requests to rename a non-open file
    pub async fn ask_rename_unopened_file(
        &mut self,
        from: String,
        to: String,
    ) -> Result<(), FileAskError> {
        let result = self
            .ask(Msg::from(Content::DoRenameUnopenedFile(
                DoRenameUnopenedFileArgs { from, to },
            )))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap().content {
            Content::UnopenedFileRenamed(_) => Ok(()),
            x => Err(make_file_ask_error(x)),
        }
    }

    /// Requests to remove an open file
    pub async fn ask_remove_file(
        &mut self,
        file: &mut RemoteFile,
    ) -> Result<(), FileAskError> {
        let result = self
            .ask(Msg::from(Content::DoRemoveFile(DoRemoveFileArgs {
                id: file.id,
                sig: file.sig,
            })))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap().content {
            Content::FileRemoved(FileRemovedArgs { sig }) => {
                file.sig = sig;
                Ok(())
            }
            x => Err(make_file_ask_error(x)),
        }
    }

    /// Requests to remove a non-open file
    pub async fn ask_remove_unopened_file(
        &mut self,
        path: String,
    ) -> Result<(), FileAskError> {
        let result = self
            .ask(Msg::from(Content::DoRemoveUnopenedFile(
                DoRemoveUnopenedFileArgs { path },
            )))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap().content {
            Content::UnopenedFileRemoved(_) => Ok(()),
            x => Err(make_file_ask_error(x)),
        }
    }

    /// Requests the full contents of a file on the server
    pub async fn ask_read_file(
        &mut self,
        file: &RemoteFile,
    ) -> Result<Vec<u8>, FileAskError> {
        let result = self
            .ask(Msg::from(Content::DoReadFile(DoReadFileArgs {
                id: file.id,
                sig: file.sig,
            })))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap().content {
            Content::FileContents(args) => Ok(args.data),
            x => Err(make_file_ask_error(x)),
        }
    }

    /// Requests to write the contents of a file on the server
    pub async fn ask_write_file(
        &mut self,
        file: &mut RemoteFile,
        contents: &[u8],
    ) -> Result<(), FileAskError> {
        let result = self
            .ask(Msg::from(Content::DoWriteFile(DoWriteFileArgs {
                id: file.id,
                sig: file.sig,
                data: contents.to_vec(),
            })))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap().content {
            Content::FileWritten(args) => {
                file.sig = args.sig;
                Ok(())
            }
            x => Err(make_file_ask_error(x)),
        }
    }

    /// Requests to execute a process on the server, providing support to
    /// send lines of text via stdin and reading back lines of text via
    /// stdout and stderr
    pub async fn ask_exec_proc(
        &mut self,
        command: String,
        args: Vec<String>,
    ) -> Result<RemoteProc, ExecAskError> {
        self.ask_exec_proc_with_streams(command, args, true, true, true)
            .await
    }

    /// Requests to execute a process on the server, indicating whether to
    /// ignore or use stdin, stdout, and stderr
    pub async fn ask_exec_proc_with_streams(
        &mut self,
        command: String,
        args: Vec<String>,
        stdin: bool,
        stdout: bool,
        stderr: bool,
    ) -> Result<RemoteProc, ExecAskError> {
        let result = self
            .ask(Msg::from(Content::DoExecProc(DoExecProcArgs {
                command,
                args,
                stdin,
                stdout,
                stderr,
            })))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap().content {
            Content::ProcStarted(args) => Ok(RemoteProc { id: args.id }),
            x => Err(make_exec_ask_error(x)),
        }
    }

    /// Requests to send lines of text to stdin of a remote process on the server
    pub async fn ask_write_stdin(
        &mut self,
        proc: &RemoteProc,
        input: &[u8],
    ) -> Result<(), ExecAskError> {
        let result = self
            .ask(Msg::from(Content::DoWriteStdin(DoWriteStdinArgs {
                id: proc.id,
                input: input.to_vec(),
            })))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap().content {
            Content::StdinWritten(_) => Ok(()),
            x => Err(make_exec_ask_error(x)),
        }
    }

    /// Requests to get all stdout from a remote process on the server since
    /// the last ask was made
    pub async fn ask_get_stdout(
        &mut self,
        proc: &RemoteProc,
    ) -> Result<Vec<u8>, ExecAskError> {
        let result = self
            .ask(Msg::from(Content::DoGetStdout(DoGetStdoutArgs {
                id: proc.id,
            })))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap().content {
            Content::StdoutContents(args) => Ok(args.output),
            x => Err(make_exec_ask_error(x)),
        }
    }

    /// Requests to get all stderr from a remote process on the server since
    /// the last ask was made
    pub async fn ask_get_stderr(
        &mut self,
        proc: &RemoteProc,
    ) -> Result<Vec<u8>, ExecAskError> {
        let result = self
            .ask(Msg::from(Content::DoGetStderr(DoGetStderrArgs {
                id: proc.id,
            })))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap().content {
            Content::StderrContents(args) => Ok(args.output),
            x => Err(make_exec_ask_error(x)),
        }
    }

    /// Requests to kill a remote process on the server
    pub async fn ask_proc_status(
        &mut self,
        proc: &RemoteProc,
    ) -> Result<RemoteProcStatus, ExecAskError> {
        let result = self
            .ask(Msg::from(Content::DoGetProcStatus(DoGetProcStatusArgs {
                id: proc.id,
            })))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap().content {
            Content::ProcStatus(args) => Ok(From::from(args)),
            x => Err(make_exec_ask_error(x)),
        }
    }

    /// Requests to kill a remote process on the server
    pub async fn ask_proc_kill(
        &mut self,
        proc: &RemoteProc,
    ) -> Result<RemoteProcStatus, ExecAskError> {
        let result = self
            .ask(Msg::from(Content::DoKillProc(DoKillProcArgs {
                id: proc.id,
            })))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap().content {
            Content::ProcStatus(args) if args.is_alive => {
                Err(ExecAskError::FailedToKill)
            }
            Content::ProcStatus(x) => Ok(From::from(x)),
            x => Err(make_exec_ask_error(x)),
        }
    }

    /// Requests internal state of server
    pub async fn ask_internal_debug(&mut self) -> Result<Vec<u8>, AskError> {
        let result = self
            .ask(Msg::from(Content::InternalDebug(InternalDebugArgs {
                input: vec![],
                output: vec![],
            })))
            .await?;

        match result.content {
            Content::InternalDebug(InternalDebugArgs { output, .. }) => {
                Ok(output)
            }
            x => Err(make_ask_error(x)),
        }
    }
}

fn make_file_ask_error(x: Content) -> FileAskError {
    match x {
        Content::IoError(args) => FileAskError::IoError(args.into()),
        x => From::from(make_ask_error(x)),
    }
}

fn make_exec_ask_error(x: Content) -> ExecAskError {
    match x {
        Content::IoError(args) => ExecAskError::IoError(args.into()),
        x => From::from(make_ask_error(x)),
    }
}

fn make_ask_error(x: Content) -> AskError {
    match x {
        content => AskError::InvalidResponse { content },
    }
}
