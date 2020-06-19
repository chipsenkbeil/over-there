use super::{
    error::{AskError, ExecAskError, FileAskError, SendError},
    file::RemoteFile,
    proc::RemoteProc,
    state::ClientState,
};
use crate::core::{
    event::{AddrEventManager, EventManager},
    msg::{
        content::{
            reply::{self, *},
            request::{self, *},
            Reply, ReplyError, Request,
        },
        Msg,
    },
};
use crate::utils::Either;
use log::{error, trace};
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
    pub async fn ask(&mut self, request: Request) -> Result<Reply, AskError> {
        let timeout = self.timeout;
        let (tx, rx) = oneshot::channel::<Result<Reply, AskError>>();
        let msg = Msg::from(request);

        // Assign a synchronous callback that uses the oneshot channel to
        // get back the result
        self.state.lock().await.callback_manager.add_callback(
            msg.header.id,
            |reply| {
                // NOTE: We handle errors like IO further downstream, so
                //       only extract the generic error here
                let result =
                    if let Reply::Error(ReplyError::Generic(x)) = &reply {
                        tx.send(Err(AskError::Failure { msg: x.to_string() }))
                    } else {
                        tx.send(Ok(reply.clone()))
                    };

                if result.is_err() {
                    error!("Failed to trigger callback: {:?}", reply);
                }
            },
        );

        // Send the msg and report back an error if it occurs
        self.send_msg(msg).await.map_err(AskError::from)?;

        tokio::time::timeout(timeout, rx)
            .await
            .map_err(|_| AskError::Timeout)?
            .map_err(|_| AskError::CallbackLost)?
    }

    /// Sends a msg to the server, not expecting a response
    pub async fn tell(&mut self, request: Request) -> Result<(), SendError> {
        self.send_msg(Msg::from(request)).await
    }

    async fn send_msg(&mut self, msg: Msg) -> Result<(), SendError> {
        trace!("Sending to {}: {:?}", self.remote_addr, msg);

        let data = msg.to_vec().map_err(|_| SendError::EncodingFailed)?;
        match &mut self.event_manager {
            Either::Left(m) => {
                m.send(data).await.map_err(|_| SendError::SendFailed)
            }
            Either::Right(m) => m
                .send_to(data, self.remote_addr)
                .await
                .map_err(|_| SendError::SendFailed),
        }
    }

    /// Requests heartbeat from the server
    pub async fn ask_heartbeat(&mut self) -> Result<(), AskError> {
        match self.ask(Request::Heartbeat).await? {
            Reply::Heartbeat => Ok(()),
            x => Err(make_ask_error(x)),
        }
    }

    /// Requests the version from the server
    pub async fn ask_version(
        &mut self,
    ) -> Result<reply::VersionArgs, AskError> {
        match self.ask(Request::Version).await? {
            Reply::Version(args) => Ok(args),
            x => Err(make_ask_error(x)),
        }
    }

    /// Requests the capabilities from the server
    pub async fn ask_capabilities(
        &mut self,
    ) -> Result<reply::CapabilitiesArgs, AskError> {
        match self.ask(Request::Capabilities).await? {
            Reply::Capabilities(args) => Ok(args),
            x => Err(make_ask_error(x)),
        }
    }

    /// Requests to create a new directory
    pub async fn ask_create_dir(
        &mut self,
        path: String,
        include_components: bool,
    ) -> Result<DirCreatedArgs, FileAskError> {
        let result = self
            .ask(Request::CreateDir(CreateDirArgs {
                path,
                include_components,
            }))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap() {
            Reply::DirCreated(args) => Ok(args),
            x => Err(make_file_ask_error(x)),
        }
    }

    /// Requests to rename an existing directory
    pub async fn ask_rename_dir(
        &mut self,
        from: String,
        to: String,
    ) -> Result<DirRenamedArgs, FileAskError> {
        let result = self
            .ask(Request::RenameDir(RenameDirArgs { from, to }))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap() {
            Reply::DirRenamed(args) => Ok(args),
            x => Err(make_file_ask_error(x)),
        }
    }

    /// Requests to remove an existing directory
    pub async fn ask_remove_dir(
        &mut self,
        path: String,
        non_empty: bool,
    ) -> Result<DirRemovedArgs, FileAskError> {
        let result = self
            .ask(Request::RemoveDir(RemoveDirArgs { path, non_empty }))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap() {
            Reply::DirRemoved(args) => Ok(args),
            x => Err(make_file_ask_error(x)),
        }
    }

    /// Requests to get a list of the root directory's contents on the server
    pub async fn ask_list_root_dir_contents(
        &mut self,
    ) -> Result<DirContentsListArgs, FileAskError> {
        self.ask_list_dir_contents(String::from(".")).await
    }

    /// Requests to get a list of a directory's contents on the server
    pub async fn ask_list_dir_contents(
        &mut self,
        path: String,
    ) -> Result<DirContentsListArgs, FileAskError> {
        let result = self
            .ask(Request::ListDirContents(ListDirContentsArgs { path }))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap() {
            Reply::DirContentsList(args) => Ok(args),
            x => Err(make_file_ask_error(x)),
        }
    }

    /// Requests to open a file for reading/writing on the server,
    /// creating the file if it does not exist
    pub async fn ask_open_file(
        &mut self,
        path: String,
    ) -> Result<FileOpenedArgs, FileAskError> {
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
    ) -> Result<FileOpenedArgs, FileAskError> {
        let result = self
            .ask(Request::OpenFile(OpenFileArgs {
                path: path.clone(),
                create_if_missing: create,
                write_access: write,
                read_access: read,
            }))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap() {
            Reply::FileOpened(args) => Ok(args),
            x => Err(make_file_ask_error(x)),
        }
    }

    /// Requests to close an open file
    pub async fn ask_close_file(
        &mut self,
        file: &RemoteFile,
    ) -> Result<FileClosedArgs, FileAskError> {
        let result = self
            .ask(Request::CloseFile(CloseFileArgs {
                id: file.id,
                sig: file.sig,
            }))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap() {
            Reply::FileClosed(args) => Ok(args),
            x => Err(make_file_ask_error(x)),
        }
    }

    /// Requests to rename an open file
    pub async fn ask_rename_file(
        &mut self,
        file: &mut RemoteFile,
        to: String,
    ) -> Result<FileRenamedArgs, FileAskError> {
        let result = self
            .ask(Request::RenameFile(RenameFileArgs {
                id: file.id,
                sig: file.sig,
                to,
            }))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap() {
            Reply::FileRenamed(args) => {
                file.sig = args.sig;
                Ok(args)
            }
            x => Err(make_file_ask_error(x)),
        }
    }

    /// Requests to rename a non-open file
    pub async fn ask_rename_unopened_file(
        &mut self,
        from: String,
        to: String,
    ) -> Result<UnopenedFileRenamedArgs, FileAskError> {
        let result = self
            .ask(Request::RenameUnopenedFile(RenameUnopenedFileArgs {
                from,
                to,
            }))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap() {
            Reply::UnopenedFileRenamed(args) => Ok(args),
            x => Err(make_file_ask_error(x)),
        }
    }

    /// Requests to remove an open file
    pub async fn ask_remove_file(
        &mut self,
        file: &mut RemoteFile,
    ) -> Result<FileRemovedArgs, FileAskError> {
        let result = self
            .ask(Request::RemoveFile(RemoveFileArgs {
                id: file.id,
                sig: file.sig,
            }))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap() {
            Reply::FileRemoved(args) => {
                file.sig = args.sig;
                Ok(args)
            }
            x => Err(make_file_ask_error(x)),
        }
    }

    /// Requests to remove a non-open file
    pub async fn ask_remove_unopened_file(
        &mut self,
        path: String,
    ) -> Result<UnopenedFileRemovedArgs, FileAskError> {
        let result = self
            .ask(Request::RemoveUnopenedFile(RemoveUnopenedFileArgs { path }))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap() {
            Reply::UnopenedFileRemoved(args) => Ok(args),
            x => Err(make_file_ask_error(x)),
        }
    }

    /// Requests the full contents of a file on the server
    pub async fn ask_read_file(
        &mut self,
        file: &RemoteFile,
    ) -> Result<FileContentsArgs, FileAskError> {
        let result = self
            .ask(Request::ReadFile(ReadFileArgs {
                id: file.id,
                sig: file.sig,
            }))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap() {
            Reply::FileContents(args) => Ok(args),
            x => Err(make_file_ask_error(x)),
        }
    }

    /// Requests to write the contents of a file on the server
    pub async fn ask_write_file(
        &mut self,
        file: &mut RemoteFile,
        contents: &[u8],
    ) -> Result<FileWrittenArgs, FileAskError> {
        let result = self
            .ask(Request::WriteFile(WriteFileArgs {
                id: file.id,
                sig: file.sig,
                contents: contents.to_vec(),
            }))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap() {
            Reply::FileWritten(args) => {
                file.sig = args.sig;
                Ok(args)
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
    ) -> Result<ProcStartedArgs, ExecAskError> {
        self.ask_exec_proc_with_options(command, args, true, true, true, None)
            .await
    }

    /// Requests to execute a process on the server, providing support to
    /// send lines of text via stdin and reading back lines of text via
    /// stdout and stderr
    pub async fn ask_exec_proc_with_current_dir(
        &mut self,
        command: String,
        args: Vec<String>,
        current_dir: String,
    ) -> Result<ProcStartedArgs, ExecAskError> {
        self.ask_exec_proc_with_options(
            command,
            args,
            true,
            true,
            true,
            Some(current_dir),
        )
        .await
    }

    /// Requests to execute a process on the server, indicating whether to
    /// ignore or use stdin, stdout, and stderr
    pub async fn ask_exec_proc_with_options(
        &mut self,
        command: String,
        args: Vec<String>,
        stdin: bool,
        stdout: bool,
        stderr: bool,
        current_dir: Option<String>,
    ) -> Result<ProcStartedArgs, ExecAskError> {
        let result = self
            .ask(Request::ExecProc(ExecProcArgs {
                command,
                args,
                stdin,
                stdout,
                stderr,
                current_dir,
            }))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap() {
            Reply::ProcStarted(args) => Ok(args),
            x => Err(make_exec_ask_error(x)),
        }
    }

    /// Requests to send lines of text to stdin of a remote process on the server
    pub async fn ask_write_proc_stdin(
        &mut self,
        proc: &RemoteProc,
        input: &[u8],
    ) -> Result<ProcStdinWrittenArgs, ExecAskError> {
        let result = self
            .ask(Request::WriteProcStdin(WriteProcStdinArgs {
                id: proc.id,
                input: input.to_vec(),
            }))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap() {
            Reply::ProcStdinWritten(args) => Ok(args),
            x => Err(make_exec_ask_error(x)),
        }
    }

    /// Requests to get all stdout from a remote process on the server since
    /// the last ask was made
    pub async fn ask_read_proc_stdout(
        &mut self,
        proc: &RemoteProc,
    ) -> Result<ProcStdoutContentsArgs, ExecAskError> {
        let result = self
            .ask(Request::ReadProcStdout(ReadProcStdoutArgs { id: proc.id }))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap() {
            Reply::ProcStdoutContents(args) => Ok(args),
            x => Err(make_exec_ask_error(x)),
        }
    }

    /// Requests to get all stderr from a remote process on the server since
    /// the last ask was made
    pub async fn ask_read_proc_stderr(
        &mut self,
        proc: &RemoteProc,
    ) -> Result<ProcStderrContentsArgs, ExecAskError> {
        let result = self
            .ask(Request::ReadProcStderr(ReadProcStderrArgs { id: proc.id }))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap() {
            Reply::ProcStderrContents(args) => Ok(args),
            x => Err(make_exec_ask_error(x)),
        }
    }

    /// Requests to read the status of a remote process on the server
    pub async fn ask_read_proc_status(
        &mut self,
        proc: &RemoteProc,
    ) -> Result<ProcStatusArgs, ExecAskError> {
        let result = self
            .ask(Request::ReadProcStatus(ReadProcStatusArgs { id: proc.id }))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap() {
            Reply::ProcStatus(args) => Ok(args),
            x => Err(make_exec_ask_error(x)),
        }
    }

    /// Requests to kill a remote process on the server
    pub async fn ask_proc_kill(
        &mut self,
        proc: &RemoteProc,
    ) -> Result<ProcKilledArgs, ExecAskError> {
        let result = self
            .ask(Request::KillProc(KillProcArgs { id: proc.id }))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap() {
            Reply::ProcKilled(args) => Ok(args),
            x => Err(make_exec_ask_error(x)),
        }
    }

    /// Requests internal state of server
    pub async fn ask_internal_debug(
        &mut self,
    ) -> Result<reply::InternalDebugArgs, AskError> {
        let result = self
            .ask(Request::InternalDebug(request::InternalDebugArgs {
                input: vec![],
            }))
            .await?;

        match result {
            Reply::InternalDebug(args) => Ok(args),
            x => Err(make_ask_error(x)),
        }
    }
}

fn make_file_ask_error(x: Reply) -> FileAskError {
    match x {
        Reply::Error(ReplyError::Io(args)) => {
            FileAskError::IoError(args.into())
        }
        x => From::from(make_ask_error(x)),
    }
}

fn make_exec_ask_error(x: Reply) -> ExecAskError {
    match x {
        Reply::Error(ReplyError::Io(args)) => {
            ExecAskError::IoError(args.into())
        }
        x => From::from(make_ask_error(x)),
    }
}

fn make_ask_error(reply: Reply) -> AskError {
    AskError::InvalidResponse { reply }
}
