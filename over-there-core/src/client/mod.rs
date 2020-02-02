mod connect;
pub mod file;
pub mod future;
pub mod proc;
pub mod state;

use crate::msg::{
    content::{
        capabilities::Capability,
        io::{exec::*, file::*},
        Content,
    },
    Msg,
};
use file::RemoteFile;
use future::{AskFuture, AskFutureState};
use log::trace;
use over_there_auth::{Signer, Verifier};
use over_there_crypto::{Decrypter, Encrypter};
use over_there_derive::Error;
use over_there_transport::{
    net, TcpStreamTransceiverError, TransceiverThread, UdpStreamTransceiverError,
};
use over_there_utils::Delay;
use proc::RemoteProc;
use std::io;
use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TellError {
    EncodingFailed,
    SendFailed,
}

impl From<AskError> for Option<TellError> {
    fn from(error: AskError) -> Self {
        match error {
            AskError::EncodingFailed => Some(TellError::EncodingFailed),
            AskError::SendFailed => Some(TellError::SendFailed),
            _ => None,
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AskError {
    Failure { msg: String },
    InvalidResponse { content: Content },
    Timeout,
    EncodingFailed,
    SendFailed,
}

impl From<TellError> for AskError {
    fn from(error: TellError) -> Self {
        match error {
            TellError::EncodingFailed => Self::EncodingFailed,
            TellError::SendFailed => Self::SendFailed,
        }
    }
}

#[derive(Debug, Error)]
pub enum FileAskError {
    GeneralAskFailed(AskError),
    IoError(io::Error),
    FileSignatureChanged { id: u32 },
}

impl From<AskError> for FileAskError {
    fn from(error: AskError) -> Self {
        Self::GeneralAskFailed(error)
    }
}

impl From<io::Error> for FileAskError {
    fn from(error: io::Error) -> Self {
        Self::IoError(error)
    }
}

#[derive(Debug, Error)]
pub enum ExecAskError {
    GeneralAskFailed(AskError),
    IoError(io::Error),
}

impl From<AskError> for ExecAskError {
    fn from(error: AskError) -> Self {
        Self::GeneralAskFailed(error)
    }
}

impl From<io::Error> for ExecAskError {
    fn from(error: io::Error) -> Self {
        Self::IoError(error)
    }
}

pub struct Client {
    state: Arc<Mutex<state::ClientState>>,

    /// Represents the address the client is connected to
    pub remote_addr: SocketAddr,

    /// Represents maximum to wait on responses before timing out
    pub timeout: Duration,

    /// Performs sending/receiving over network
    transceiver_thread: TransceiverThread<Vec<u8>, ()>,

    /// Processes incoming msg structs
    msg_thread: JoinHandle<()>,
}

impl Client {
    /// Default timeout applied to a new client for any ask made
    pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

    pub fn connect_tcp<A, B, C>(
        remote_addr: SocketAddr,
        packet_ttl: Duration,
        authenticator: A,
        bicrypter: B,
        err_callback: C,
    ) -> Result<Self, io::Error>
    where
        A: Signer + Verifier + Send + Sync + 'static,
        B: Encrypter + Decrypter + Send + Sync + 'static,
        C: Fn(TcpStreamTransceiverError) -> bool + Send + 'static,
    {
        Self::connect_using_tcp_stream(
            TcpStream::connect(remote_addr)?,
            packet_ttl,
            authenticator,
            bicrypter,
            err_callback,
        )
    }

    pub fn connect_using_tcp_stream<A, B, C>(
        stream: TcpStream,
        packet_ttl: Duration,
        authenticator: A,
        bicrypter: B,
        err_callback: C,
    ) -> Result<Self, io::Error>
    where
        A: Signer + Verifier + Send + Sync + 'static,
        B: Encrypter + Decrypter + Send + Sync + 'static,
        C: Fn(TcpStreamTransceiverError) -> bool + Send + 'static,
    {
        connect::tcp_connect(stream, packet_ttl, authenticator, bicrypter, err_callback)
    }

    pub fn connect_udp<A, B, C>(
        remote_addr: SocketAddr,
        packet_ttl: Duration,
        authenticator: A,
        bicrypter: B,
        err_callback: C,
    ) -> Result<Self, io::Error>
    where
        A: Signer + Verifier + Send + Sync + 'static,
        B: Encrypter + Decrypter + Send + Sync + 'static,
        C: Fn(UdpStreamTransceiverError) -> bool + Send + 'static,
    {
        Self::connect_using_udp_socket(
            net::udp::connect(remote_addr)?,
            remote_addr,
            packet_ttl,
            authenticator,
            bicrypter,
            err_callback,
        )
    }

    pub fn connect_using_udp_socket<A, B, C>(
        socket: UdpSocket,
        remote_addr: SocketAddr,
        packet_ttl: Duration,
        authenticator: A,
        bicrypter: B,
        err_callback: C,
    ) -> Result<Self, io::Error>
    where
        A: Signer + Verifier + Send + Sync + 'static,
        B: Encrypter + Decrypter + Send + Sync + 'static,
        C: Fn(UdpStreamTransceiverError) -> bool + Send + 'static,
    {
        connect::udp_connect(
            socket,
            remote_addr,
            packet_ttl,
            authenticator,
            bicrypter,
            err_callback,
        )
    }

    pub fn join(self) -> Result<(), Box<dyn std::error::Error>> {
        self.transceiver_thread.join()?;
        self.msg_thread
            .join()
            .map_err(|_| "Msg Process Thread Join Error")?;

        Ok(())
    }

    /// Generic ask of the server that is expecting a response
    pub async fn ask(&self, msg: Msg) -> Result<Msg, AskError> {
        let timeout = self.timeout;
        let state = Arc::new(Mutex::new(AskFutureState::new(timeout)));

        let callback_state = Arc::clone(&state);
        self.state
            .lock()
            .unwrap()
            .callback_manager
            .add_callback(msg.header.id, move |msg| {
                let mut s = callback_state.lock().unwrap();
                if let Content::Error(args) = &msg.content {
                    s.result = Some(Err(AskError::Failure {
                        msg: args.msg.to_string(),
                    }));
                } else {
                    s.result = Some(Ok(msg.clone()));
                }

                if let Some(waker) = s.waker.take() {
                    waker.wake();
                }
            });

        // Send the msg and report back an error if it occurs
        self.tell(msg).await.map_err(AskError::from)?;

        // TODO: Is there a better way to provide timing functionality for
        //       expirations than using a new thread to check?
        let delay_state = Arc::clone(&state);
        let delay = Delay::spawn(timeout, move || {
            let mut s = delay_state.lock().unwrap();

            if let Some(waker) = s.waker.take() {
                waker.wake();
            }
        });

        let result = AskFuture { state }.await;

        // Cancel the delayed timeout if it hasn't already been processed
        delay.cancel();

        result
    }

    /// Sends a msg to the server, not expecting a response
    pub async fn tell(&self, msg: Msg) -> Result<(), TellError> {
        trace!("Sending to {}: {:?}", self.remote_addr, msg);

        // TODO: Make non-blocking, would involve re-writing transport to use
        //       async implementation
        self.transceiver_thread
            .send(msg.to_vec().map_err(|_| TellError::EncodingFailed)?)
            .map_err(|_| TellError::SendFailed)
    }

    /// Requests the version from the server
    pub async fn ask_version(&self) -> Result<String, AskError> {
        let msg = self.ask(Msg::from(Content::DoGetVersion)).await?;
        match msg.content {
            Content::Version(args) => Ok(args.version),
            x => Err(make_ask_error(x)),
        }
    }

    /// Requests the capabilities from the server
    pub async fn ask_capabilities(&self) -> Result<Vec<Capability>, AskError> {
        let msg = self.ask(Msg::from(Content::DoGetCapabilities)).await?;
        match msg.content {
            Content::Capabilities(args) => Ok(args.capabilities),
            x => Err(make_ask_error(x)),
        }
    }

    /// Requests to get a list of the root directory's contents on the server
    pub async fn ask_list_root_dir_contents(&self) -> Result<Vec<DirEntry>, FileAskError> {
        self.ask_list_dir_contents(".").await
    }

    /// Requests to get a list of a directory's contents on the server
    pub async fn ask_list_dir_contents(&self, path: &str) -> Result<Vec<DirEntry>, FileAskError> {
        let result = self
            .ask(Msg::from(Content::DoListDirContents(
                DoListDirContentsArgs {
                    path: path.to_string(),
                },
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
    pub async fn ask_open_file(&self, path: &str) -> Result<RemoteFile, FileAskError> {
        self.ask_open_file_with_options(path, true, true, true)
            .await
    }

    /// Requests to open a file on the server, opening using the provided options
    pub async fn ask_open_file_with_options(
        &self,
        path: &str,
        create: bool,
        write: bool,
        read: bool,
    ) -> Result<RemoteFile, FileAskError> {
        let result = self
            .ask(Msg::from(Content::DoOpenFile(DoOpenFileArgs {
                path: path.to_string(),
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
                path: path.to_string(),
            }),
            x => Err(make_file_ask_error(x)),
        }
    }

    /// Requests the full contents of a file on the server
    pub async fn ask_read_file(&self, file: &RemoteFile) -> Result<Vec<u8>, FileAskError> {
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
        &self,
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
    pub async fn ask_exec(
        &self,
        command: String,
        args: Vec<String>,
    ) -> Result<RemoteProc, ExecAskError> {
        self.ask_exec_with_streams(command, args, true, true, true)
            .await
    }

    /// Requests to execute a process on the server, indicating whether to
    /// ignore or use stdin, stdout, and stderr
    pub async fn ask_exec_with_streams(
        &self,
        command: String,
        args: Vec<String>,
        stdin: bool,
        stdout: bool,
        stderr: bool,
    ) -> Result<RemoteProc, ExecAskError> {
        let result = self
            .ask(Msg::from(Content::DoExec(DoExecArgs {
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
            Content::ExecStarted(args) => Ok(RemoteProc { id: args.id }),
            x => Err(make_exec_ask_error(x)),
        }
    }

    /// Requests to send lines of text to stdin of a remote process on the server
    pub async fn tell_exec_stdin(
        &self,
        proc: &mut RemoteProc,
        input: Vec<u8>,
    ) -> Result<(), ExecAskError> {
        self.tell(Msg::from(Content::DoExecStdin(DoExecStdinArgs {
            id: proc.id,
            input,
        })))
        .await
        .map_err(|e| From::from(AskError::from(e)))
    }

    /// Requests to get all stdout from a remote process on the server since
    /// the last ask was made
    pub async fn ask_exec_stdout(&self, proc: &RemoteProc) -> Result<Vec<u8>, ExecAskError> {
        let result = self
            .ask(Msg::from(Content::DoGetExecStdout(DoGetExecStdoutArgs {
                id: proc.id,
            })))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap().content {
            Content::ExecStdoutContents(args) => Ok(args.output),
            x => Err(make_exec_ask_error(x)),
        }
    }

    /// Requests to get all stderr from a remote process on the server since
    /// the last ask was made
    pub async fn ask_exec_stderr(&self, proc: &RemoteProc) -> Result<Vec<u8>, ExecAskError> {
        let result = self
            .ask(Msg::from(Content::DoGetExecStderr(DoGetExecStderrArgs {
                id: proc.id,
            })))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap().content {
            Content::ExecStderrContents(args) => Ok(args.output),
            x => Err(make_exec_ask_error(x)),
        }
    }

    /// Requests to kill a remote process on the server
    pub async fn ask_exec_kill(&self, proc: &RemoteProc) -> Result<u32, ExecAskError> {
        let result = self
            .ask(Msg::from(Content::DoExecKill(DoExecKillArgs {
                id: proc.id,
            })))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap().content {
            Content::ExecExit(args) => Ok(args.exit_code),
            x => Err(make_exec_ask_error(x)),
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
