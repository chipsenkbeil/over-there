pub mod error;
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
use future::{AskFuture, AskFutureState};
use log::{error, trace, warn};
use over_there_auth::{Signer, Verifier};
use over_there_crypto::{Decrypter, Encrypter};
use over_there_utils::Delay;
use over_there_wire::{self as wire, InboundWire, InboundWireError, NetTransmission, OutboundWire};
use proc::RemoteProc;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::{
    io,
    net::{TcpStream, UdpSocket},
    runtime::Runtime,
    task,
};

impl<S, V, E, D> Communicator<S, V, E, D>
where
    S: Signer,
    V: Verifier,
    E: Encrypter,
    D: Decrypter,
{
    /// Starts actively listening for msgs via the specified transport medium
    pub async fn connect(self, transport: Transport) -> io::Result<Client> {
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
                let event_handle = handle.spawn(async {
                    loop {
                        let result = inbound_wire
                            .async_recv(|buf| {
                                use futures::future::FutureExt;
                                use io::AsyncReadExt;
                                stream
                                    .read(buf)
                                    .map(|res| res.map(|size| (size, remote_addr)))
                            })
                            .await;
                        if !process_recv(&mut state, result).await {
                            break;
                        }
                    }
                });

                Ok(Client {
                    state,
                    runtime,
                    event_handle,
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
                let event_handle = handle.spawn(async {
                    loop {
                        let result = inbound_wire.async_recv(|buf| socket.recv_from(buf)).await;
                        if !process_recv(&mut state, result).await {
                            break;
                        }
                    }
                });

                Ok(Client {
                    state,
                    runtime,
                    event_handle,
                    remote_addr,
                    timeout: Client::DEFAULT_TIMEOUT,
                })
            }
        }
    }
}

/// Process result of receiving data, indicating whether should continue
/// processing additional data
async fn process_recv(
    state: &mut state::ClientState,
    result: Result<Option<(Vec<u8>, SocketAddr)>, InboundWireError>,
) -> bool {
    match result {
        Ok(None) => true,
        Ok(Some((data, addr))) => {
            trace!("Incoming data of size {}", data.len());
            if let Ok(msg) = Msg::from_slice(&data) {
                trace!("Forwarding {:?} using {:?}", msg, addr);
                // TODO: Invoke callback
                true
            } else {
                warn!("Discarding data of size {} as not valid msg", data.len());
                true
            }
        }
        Err(x) => match x {
            InboundWireError::IO(x) => {
                error!("Fatal IO on socket: {}", x);
                false
            }
            InboundWireError::InputProcessor(x) => {
                error!("Process error on socket: {}", x);
                true
            }
        },
    }
}

/// Represents a client after connecting to an endpoint
pub struct Client {
    state: state::ClientState,

    /// Used to spawn jobs when communicating with the server
    runtime: Runtime,

    /// Primary event handle processing incoming msgs
    event_handle: task::JoinHandle<()>,

    /// Represents the address the client is connected to
    remote_addr: SocketAddr,

    /// Represents maximum to wait on responses before timing out
    timeout: Duration,
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

    pub fn timeout(&self) -> Duration {
        self.timeout
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
