pub mod error;
mod event;
pub mod file;
pub mod future;
pub mod proc;
pub mod state;

use crate::{
    msg::{
        content::{
            capabilities::Capability,
            io::{file::*, proc::*},
            Content,
        },
        Msg,
    },
    Communicator, Transport,
};
use error::{AskError, ExecAskError, FileAskError, TellError};
use file::RemoteFile;
use log::trace;
use over_there_auth::{Signer, Verifier};
use over_there_crypto::{Decrypter, Encrypter};
use over_there_wire::{self as wire, InboundWire, NetTransmission, OutboundWire};
use proc::RemoteProc;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::{
    io,
    net::{TcpStream, UdpSocket},
    runtime::Runtime,
    sync::{mpsc, oneshot},
    task,
};

impl<S, V, E, D> Communicator<S, V, E, D>
where
    S: Signer,
    V: Verifier + Send + 'static,
    E: Encrypter,
    D: Decrypter + Send + 'static,
{
    /// Starts actively listening for msgs via the specified transport medium
    pub async fn connect(self, transport: Transport, buffer: usize) -> io::Result<Client> {
        let runtime = Runtime::new()?;
        let handle = runtime.handle();
        let mut state = state::ClientState::default();

        match transport {
            Transport::Tcp(addrs) => {
                // NOTE: Tokio does not support &[SocketAddr] -> ToSocketAddrs,
                //       so we have to loop through manually
                // See https://github.com/tokio-rs/tokio/pull/1760#discussion_r379120864
                let stream = {
                    let mut stream = None;
                    for addr in addrs.iter() {
                        let result = TcpStream::connect(addr).await;
                        if result.is_ok() {
                            stream = result.ok();
                            break;
                        }
                    }
                    stream.ok_or(io::Error::from(io::ErrorKind::ConnectionRefused))?
                };
                let remote_addr = stream.peer_addr()?;
                let inbound_wire = InboundWire::new(
                    NetTransmission::TcpEthernet.into(),
                    self.packet_ttl,
                    self.verifier,
                    self.decrypter,
                );
                let outbound_wire = OutboundWire::new(
                    NetTransmission::TcpEthernet.into(),
                    self.signer,
                    self.encrypter,
                );

                let event::Loops {
                    send_handle,
                    event_handle,
                    tx,
                } = event::spawn_tcp_loops(
                    handle.clone(),
                    buffer,
                    inbound_wire,
                    stream,
                    remote_addr,
                    state,
                );

                Ok(Client {
                    state,
                    runtime,
                    event_handle,
                    send_handle,
                    tx,
                    remote_addr,
                    timeout: Client::DEFAULT_TIMEOUT,
                })
            }
            Transport::Udp(addrs) => {
                // NOTE: Tokio does not support &[SocketAddr] -> ToSocketAddrs,
                //       so we have to loop through manually
                // See https://github.com/tokio-rs/tokio/pull/1760#discussion_r379120864
                let (socket, remote_addr) = {
                    let mut socketAndAddr = None;
                    for addr in addrs.iter() {
                        let result = wire::net::udp::connect(*addr);
                        if result.is_ok() {
                            socketAndAddr = result.ok().map(|s| (s, *addr));
                            break;
                        }
                    }

                    // TODO: Use DNS resolver to evaluate addresses
                    // NOTE: Must use Handle::enter to provide proper runtime when
                    //       using UdpSocket::from_std
                    handle.enter(|| {
                        socketAndAddr
                            .ok_or(io::Error::from(io::ErrorKind::ConnectionRefused))
                            .and_then(|(s, addr)| UdpSocket::from_std(s).map(|s| (s, addr)))
                    })?
                };

                let addr = socket.local_addr()?;
                let transmission = NetTransmission::udp_from_addr(addr);

                let inbound_wire = InboundWire::new(
                    transmission.into(),
                    self.packet_ttl,
                    self.verifier,
                    self.decrypter,
                );
                let outbound_wire =
                    OutboundWire::new(transmission.into(), self.signer, self.encrypter);

                let event::Loops {
                    send_handle,
                    event_handle,
                    tx,
                } = event::spawn_udp_loops(handle.clone(), buffer, inbound_wire, socket, state);

                Ok(Client {
                    state,
                    runtime,
                    event_handle,
                    send_handle,
                    tx,
                    remote_addr,
                    timeout: Client::DEFAULT_TIMEOUT,
                })
            }
        }
    }
}

/// Represents a client after connecting to an endpoint
pub struct Client {
    state: state::ClientState,

    /// Used to spawn jobs when communicating with the server
    runtime: Runtime,

    /// Primary event handle processing incoming msgs
    event_handle: task::JoinHandle<()>,

    /// Primary send handle processing outgoing msgs
    send_handle: task::JoinHandle<()>,

    /// Means to send new outbound msgs
    tx: mpsc::Sender<(Vec<u8>, SocketAddr)>,

    /// Represents the address the client is connected to
    remote_addr: SocketAddr,

    /// Represents maximum to wait on responses before timing out
    pub timeout: Duration,
}

impl Client {
    /// Default timeout applied to a new client for any ask made
    pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

    pub fn event_handle(&self) -> &task::JoinHandle<()> {
        &self.event_handle
    }

    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    pub fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }

    /// Generic ask of the server that is expecting a response
    pub async fn ask(&self, msg: Msg) -> Result<Msg, AskError> {
        let timeout = self.timeout;
        let (tx, rx) = oneshot::channel::<Result<Msg, AskError>>();

        // Assign a synchronous callback that uses the oneshot channel to
        // get back the result
        self.state
            .callback_manager
            .add_callback(msg.header.id, |msg| {
                if let Content::Error(args) = &msg.content {
                    tx.send(Err(AskError::Failure {
                        msg: args.msg.to_string(),
                    }));
                } else {
                    tx.send(Ok(msg.clone()));
                }
            });

        // Send the msg and report back an error if it occurs
        self.tell(msg).await.map_err(AskError::from)?;

        tokio::time::timeout(timeout, rx)
            .await
            .map_err(|_| AskError::Timeout)?
            .map_err(|_| AskError::CallbackLost)?
    }

    /// Sends a msg to the server, not expecting a response
    pub async fn tell(&self, msg: Msg) -> Result<(), TellError> {
        trace!("Sending to {}: {:?}", self.remote_addr, msg);

        self.tx
            .send((
                msg.to_vec().map_err(|_| TellError::EncodingFailed)?,
                self.remote_addr,
            ))
            .await
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
        self.ask_list_dir_contents(String::from(".")).await
    }

    /// Requests to get a list of a directory's contents on the server
    pub async fn ask_list_dir_contents(&self, path: String) -> Result<Vec<DirEntry>, FileAskError> {
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
    pub async fn ask_open_file(&self, path: String) -> Result<RemoteFile, FileAskError> {
        self.ask_open_file_with_options(path, true, true, true)
            .await
    }

    /// Requests to open a file on the server, opening using the provided options
    pub async fn ask_open_file_with_options(
        &self,
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
    pub async fn ask_exec_proc(
        &self,
        command: String,
        args: Vec<String>,
    ) -> Result<RemoteProc, ExecAskError> {
        self.ask_exec_proc_with_streams(command, args, true, true, true)
            .await
    }

    /// Requests to execute a process on the server, indicating whether to
    /// ignore or use stdin, stdout, and stderr
    pub async fn ask_exec_proc_with_streams(
        &self,
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
        &self,
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
    pub async fn ask_get_stdout(&self, proc: &RemoteProc) -> Result<Vec<u8>, ExecAskError> {
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
    pub async fn ask_get_stderr(&self, proc: &RemoteProc) -> Result<Vec<u8>, ExecAskError> {
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
    pub async fn ask_proc_kill(&self, proc: &RemoteProc) -> Result<(), ExecAskError> {
        let result = self
            .ask(Msg::from(Content::DoKillProc(DoKillProcArgs {
                id: proc.id,
            })))
            .await;

        if let Err(x) = result {
            return Err(From::from(x));
        }

        match result.unwrap().content {
            Content::ProcStatus(args) if args.is_alive => Err(ExecAskError::FailedToKill),
            Content::ProcStatus(_) => Ok(()),
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
