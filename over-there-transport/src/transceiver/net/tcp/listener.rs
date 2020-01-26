use crate::transceiver::{
    net::{self, Data, DataAndAddr, NetListener, NetResponder},
    TransceiverContext, TransceiverThread,
};
use over_there_auth::{Signer, Verifier};
use over_there_crypto::{Decrypter, Encrypter};
use over_there_derive::Error;
use std::collections::HashMap;
use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread::{self};
use std::time::Duration;

#[derive(Debug, Error)]
pub enum TcpListenerTransceiverError {
    AcceptError(io::Error),
    TcpStreamError(net::tcp::stream::TcpStreamTransceiverError),
    Disconnected,
}

pub struct TcpListenerTransceiver<A, B>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
{
    pub listener: TcpListener,
    ctx: TransceiverContext<A, B>,
}

impl<A, B> TcpListenerTransceiver<A, B>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
{
    pub fn new(listener: TcpListener, ctx: TransceiverContext<A, B>) -> Self {
        Self { listener, ctx }
    }
}

impl<A, B> NetListener for TcpListenerTransceiver<A, B>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
{
    type Error = TcpListenerTransceiverError;
    type Responder = NetResponder;

    fn spawn<C, D>(
        self,
        sleep_duration: Duration,
        callback: C,
        err_callback: D,
    ) -> io::Result<TransceiverThread<DataAndAddr, ()>>
    where
        C: Fn(Data, Self::Responder) + Send + 'static,
        D: Fn(Self::Error) -> bool + Send + 'static,
    {
        // NOTE: Listener MUST be nonblocking, otherwise accept will stall
        self.listener.set_nonblocking(true)?;

        spawn(
            self.listener,
            self.ctx,
            sleep_duration,
            callback,
            err_callback,
        )
    }
}

fn spawn<A, B, C, D>(
    listener: TcpListener,
    mut ctx: TransceiverContext<A, B>,
    sleep_duration: Duration,
    callback: C,
    err_callback: D,
) -> Result<TransceiverThread<DataAndAddr, ()>, io::Error>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
    C: Fn(Data, NetResponder) + Send + 'static,
    D: Fn(TcpListenerTransceiverError) -> bool + Send + 'static,
{
    // Must be non-blocking so we can accept new connections within the same
    // thread as sending/receiving data
    listener.set_nonblocking(true)?;

    let (tx, rx) = mpsc::channel::<DataAndAddr>();

    let handle = thread::spawn(move || {
        let mut connections = HashMap::new();
        loop {
            if let Err(e) = process(&listener, &mut connections, &mut ctx, &rx, &callback) {
                if !err_callback(e) {
                    break;
                }
            }
            thread::sleep(sleep_duration);
        }
    });

    Ok(TransceiverThread { handle, tx })
}

fn process<A, B, C>(
    listener: &TcpListener,
    connections: &mut HashMap<SocketAddr, (TcpStream, mpsc::Sender<Data>, mpsc::Receiver<Data>)>,
    ctx: &mut TransceiverContext<A, B>,
    send_rx: &mpsc::Receiver<DataAndAddr>,
    callback: &C,
) -> Result<(), TcpListenerTransceiverError>
where
    A: Signer + Verifier,
    B: Encrypter + Decrypter,
    C: Fn(Data, NetResponder) + Send + 'static,
{
    // Process a new connection if we have one
    match listener.accept() {
        Ok((stream, addr)) => {
            let (tx, rx) = mpsc::channel::<Data>();
            connections.insert(addr, (stream, tx, rx));
        }
        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => (),
        Err(x) => return Err(TcpListenerTransceiverError::AcceptError(x)),
    }

    // Attempt to send data on stream if there is any available
    match send_rx.try_recv() {
        Ok((data, addr)) => {
            if let Some((_, tx, _)) = connections.get(&addr) {
                // If we get an error attempting to send data through the
                // connection, we assume that the connection has been dropped
                // and that we should remove it
                if let Err(_) = tx.send(data) {
                    connections.remove(&addr);
                }
            }
        }
        Err(mpsc::TryRecvError::Empty) => (),
        Err(mpsc::TryRecvError::Disconnected) => {
            return Err(TcpListenerTransceiverError::Disconnected)
        }
    }

    // Run through all streams
    for (_addr, (stream, tx, rx)) in connections.iter_mut() {
        let ns = NetResponder { tx: tx.clone() };

        if let Err(e) = net::tcp::stream::stream_process(stream, ctx, rx, &ns, callback) {
            return Err(TcpListenerTransceiverError::TcpStreamError(e));
        }
    }

    Ok(())
}
