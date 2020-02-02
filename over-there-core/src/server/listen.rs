use crate::{
    msg::Msg,
    server::{
        action::{self, ActionError},
        state::ServerState,
        Server,
    },
};
use log::trace;
use over_there_auth::{Signer, Verifier};
use over_there_crypto::{Decrypter, Encrypter};
use over_there_transport::{
    NetListener, NetTransmission, TcpListenerTransceiver, TcpListenerTransceiverError,
    TransceiverContext, TransceiverThread, UdpTransceiver, UdpTransceiverError,
};
use std::io;
use std::net::{SocketAddr, TcpListener, UdpSocket};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

fn spawn_threads<L, C>(
    state: Arc<Mutex<ServerState>>,
    listener: L,
    err_callback: C,
) -> Result<
    (
        TransceiverThread<(Vec<u8>, SocketAddr), ()>,
        thread::JoinHandle<()>,
    ),
    io::Error,
>
where
    L: NetListener + 'static,
    C: Fn(L::Error) -> bool + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    let thread = listener.spawn(
        Duration::from_millis(1),
        move |data: Vec<u8>, responder| {
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
        .name(String::from("server-action"))
        .spawn(move || {
            loop {
                if let Ok((msg, responder)) = rx.try_recv() {
                    let s: &mut ServerState = &mut *state.lock().unwrap();
                    trace!("Processing {:?} using {:?}", msg, responder);
                    match action::execute(s, &msg, &responder) {
                        // If unknown, ignore it; if succeed, keep going
                        Ok(_) | Err(ActionError::Unknown) => (),

                        // TODO: Handle action errors?
                        Err(_) => (),
                    }
                }
            }
        })?;

    Ok((thread, handle))
}

pub fn tcp_listen<A, B, C>(
    listener: TcpListener,
    packet_ttl: Duration,
    authenticator: A,
    bicrypter: B,
    err_callback: C,
) -> Result<Server, io::Error>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
    C: Fn(TcpListenerTransceiverError) -> bool + Send + 'static,
{
    let addr = listener.local_addr()?;
    let state = Arc::new(Mutex::new(ServerState::default()));
    let stream = TcpListenerTransceiver::new(
        listener,
        TransceiverContext::new(
            NetTransmission::TcpEthernet.into(),
            packet_ttl,
            authenticator,
            bicrypter,
        ),
    );

    let (transceiver_thread, msg_thread) = spawn_threads(Arc::clone(&state), stream, err_callback)?;
    Ok(Server {
        state,
        addr,
        transceiver_thread,
        msg_thread,
    })
}

pub fn udp_listen<A, B, C>(
    socket: UdpSocket,
    packet_ttl: Duration,
    authenticator: A,
    bicrypter: B,
    err_callback: C,
) -> Result<Server, io::Error>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
    C: Fn(UdpTransceiverError) -> bool + Send + 'static,
{
    let addr = socket.local_addr()?;
    let state = Arc::new(Mutex::new(ServerState::default()));
    let ctx = TransceiverContext::new(
        if socket.local_addr()?.is_ipv4() {
            NetTransmission::UdpIpv4.into()
        } else {
            NetTransmission::UdpIpv6.into()
        },
        packet_ttl,
        authenticator,
        bicrypter,
    );
    let transceiver = UdpTransceiver::new(socket, ctx);
    let (transceiver_thread, msg_thread) =
        spawn_threads(Arc::clone(&state), transceiver, err_callback)?;

    Ok(Server {
        state,
        addr,
        transceiver_thread,
        msg_thread,
    })
}
