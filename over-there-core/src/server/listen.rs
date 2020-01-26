use crate::{
    action,
    msg::{content::ContentType, Msg},
    server::{route, state::ServerState, Server},
};
use over_there_auth::{Signer, Verifier};
use over_there_crypto::{Decrypter, Encrypter};
use over_there_transport::{
    NetListener, NetTransmission, TcpListenerTransceiver, TransceiverContext, TransceiverThread,
    UdpTransceiver,
};
use std::io;
use std::net::{SocketAddr, TcpListener, UdpSocket};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

fn spawn_threads<L>(
    state: Arc<Mutex<ServerState>>,
    listener: L,
) -> Result<
    (
        TransceiverThread<(Vec<u8>, SocketAddr), ()>,
        thread::JoinHandle<()>,
    ),
    io::Error,
>
where
    L: NetListener + 'static,
{
    let (tx, rx) = mpsc::channel();
    let thread = listener.spawn(Duration::from_millis(1), move |data: Vec<u8>, responder| {
        if let Ok(msg) = Msg::from_slice(&data) {
            // TODO: Handle send error?
            tx.send((msg, responder)).unwrap();
        }
    })?;

    let handle = thread::spawn(move || {
        loop {
            if let Ok((msg, responder)) = rx.try_recv() {
                let s: &mut ServerState = &mut *state.lock().unwrap();
                // TODO: Handle action errors?
                action::execute(
                    s,
                    &msg,
                    &responder,
                    route::route(ContentType::from(msg.content.clone())),
                )
                .unwrap();
            }
        }
    });

    Ok((thread, handle))
}

pub fn tcp_listen<A, B>(
    listener: TcpListener,
    packet_ttl: Duration,
    authenticator: A,
    bicrypter: B,
) -> Result<Server, io::Error>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
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

    let (transceiver_thread, msg_thread) = spawn_threads(Arc::clone(&state), stream)?;
    Ok(Server {
        state,
        addr,
        transceiver_thread,
        msg_thread,
    })
}

pub fn udp_listen<A, B>(
    socket: UdpSocket,
    packet_ttl: Duration,
    authenticator: A,
    bicrypter: B,
) -> Result<Server, io::Error>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
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
    let (transceiver_thread, msg_thread) = spawn_threads(Arc::clone(&state), transceiver)?;

    Ok(Server {
        state,
        addr,
        transceiver_thread,
        msg_thread,
    })
}
