use crate::{
    action,
    client::{route, state::ClientState, Client},
    msg::{content::ContentType, Msg},
};
use over_there_auth::{Signer, Verifier};
use over_there_crypto::{Decrypter, Encrypter};
use over_there_transport::{
    NetResponder, NetStream, NetTransmission, TcpStreamTransceiver, TransceiverContext,
    TransceiverThread, UdpTransceiver,
};
use std::io;
use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

fn spawn_threads<S>(
    state: Arc<Mutex<ClientState>>,
    stream: S,
) -> Result<(TransceiverThread<Vec<u8>, ()>, thread::JoinHandle<()>), io::Error>
where
    S: NetStream,
{
    let (tx, rx) = mpsc::channel();
    let thread = stream.spawn(
        Duration::from_millis(1),
        move |data: Vec<u8>, responder: NetResponder| {
            if let Ok(msg) = Msg::from_slice(&data) {
                // TODO: Handle send error?
                tx.send((msg, responder)).unwrap();
            }
        },
    )?;

    let handle = thread::spawn(move || {
        loop {
            if let Ok((msg, responder)) = rx.try_recv() {
                let s: &mut ClientState = &mut *state.lock().unwrap();
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

pub fn tcp_connect<A, B>(
    stream: TcpStream,
    packet_ttl: Duration,
    authenticator: A,
    bicrypter: B,
) -> Result<Client, io::Error>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
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
    );

    let (transceiver_thread, msg_thread) = spawn_threads(Arc::clone(&state), stream)?;
    Ok(Client {
        state,
        remote_addr,
        transceiver_thread,
        msg_thread,
    })
}

pub fn udp_connect<A, B>(
    socket: UdpSocket,
    remote_addr: SocketAddr,
    packet_ttl: Duration,
    authenticator: A,
    bicrypter: B,
) -> Result<Client, io::Error>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
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
    let (transceiver_thread, msg_thread) = spawn_threads(Arc::clone(&state), stream)?;

    Ok(Client {
        state,
        remote_addr,
        transceiver_thread,
        msg_thread,
    })
}
