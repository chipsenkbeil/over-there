use crate::{
    client::{state::ClientState, Client},
    msg::Msg,
};
use log::trace;
use over_there_auth::{Signer, Verifier};
use over_there_crypto::{Decrypter, Encrypter};
use over_there_transport::{
    NetResponder, NetStream, NetTransmission, TcpStreamTransceiver, TcpStreamTransceiverError,
    TransceiverContext, TransceiverThread, UdpStreamTransceiverError, UdpTransceiver,
};
use std::io;
use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

fn spawn_threads<S, C>(
    state: Arc<Mutex<ClientState>>,
    stream: S,
    err_callback: C,
) -> Result<(TransceiverThread<Vec<u8>, ()>, thread::JoinHandle<()>), io::Error>
where
    S: NetStream,
    C: Fn(S::Error) -> bool + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    let thread = stream.spawn(
        Duration::from_millis(1),
        move |data: Vec<u8>, responder: NetResponder| {
            trace!("Incoming data of size {}", data.len());
            if let Ok(msg) = Msg::from_slice(&data) {
                // TODO: Handle send error?
                trace!("Forwarding {:?} using {:?}", msg, responder);
                tx.send((msg, responder)).unwrap();
            }
        },
        err_callback,
    )?;

    let handle = thread::Builder::new()
        .name(String::from("client-action"))
        .spawn(move || {
            loop {
                if let Ok((msg, responder)) = rx.try_recv() {
                    let s: &mut ClientState = &mut *state.lock().unwrap();
                    // TODO: Handle action errors?
                    trace!("Processing {:?} using {:?}", msg, responder);

                    if let Some(header) = msg.parent_header.as_ref() {
                        s.callback_manager.invoke_callback(header.id, &msg)
                    }
                }
            }
        })?;

    Ok((thread, handle))
}

pub fn tcp_connect<A, B, C>(
    stream: TcpStream,
    packet_ttl: Duration,
    authenticator: A,
    bicrypter: B,
    err_callback: C,
) -> Result<Client, io::Error>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
    C: Fn(TcpStreamTransceiverError) -> bool + Send + 'static,
{
    let remote_addr = stream.peer_addr()?;
    let state = Arc::new(Mutex::new(ClientState::default()));
    let stream = TcpStreamTransceiver::new(
        stream,
        TransceiverContext::new(
            NetTransmission::TcpEthernet.into(),
            packet_ttl,
            authenticator,
            bicrypter,
        ),
    )?;

    let (transceiver_thread, msg_thread) = spawn_threads(Arc::clone(&state), stream, err_callback)?;
    Ok(Client {
        state,
        remote_addr,
        timeout: Client::DEFAULT_TIMEOUT,
        transceiver_thread,
        msg_thread,
    })
}

pub fn udp_connect<A, B, C>(
    socket: UdpSocket,
    remote_addr: SocketAddr,
    packet_ttl: Duration,
    authenticator: A,
    bicrypter: B,
    err_callback: C,
) -> Result<Client, io::Error>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
    C: Fn(UdpStreamTransceiverError) -> bool + Send + 'static,
{
    let state = Arc::new(Mutex::new(ClientState::default()));
    let ctx = TransceiverContext::new(
        if remote_addr.is_ipv4() {
            NetTransmission::UdpIpv4.into()
        } else {
            NetTransmission::UdpIpv6.into()
        },
        packet_ttl,
        authenticator,
        bicrypter,
    );
    let stream = UdpTransceiver::new(socket, ctx).connect(remote_addr)?;
    let (transceiver_thread, msg_thread) = spawn_threads(Arc::clone(&state), stream, err_callback)?;

    Ok(Client {
        state,
        remote_addr,
        timeout: Client::DEFAULT_TIMEOUT,
        transceiver_thread,
        msg_thread,
    })
}
