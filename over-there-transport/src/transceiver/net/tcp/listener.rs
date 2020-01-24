use crate::transceiver::{
    net::{self, Data, DataAndAddr, NetListener, NetResponder},
    TransceiverContext, TransceiverThread,
};
use over_there_auth::{Signer, Verifier};
use over_there_crypto::{Decrypter, Encrypter};
use std::collections::HashMap;
use std::io;
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread::{self};
use std::time::Duration;

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
    type Responder = NetResponder;

    fn spawn<C>(
        self,
        sleep_duration: Duration,
        callback: C,
    ) -> io::Result<TransceiverThread<DataAndAddr, ()>>
    where
        C: Fn(Data, Self::Responder) + Send + 'static,
    {
        spawn(self.listener, self.ctx, sleep_duration, callback)
    }
}

fn spawn<A, B, C>(
    listener: TcpListener,
    mut ctx: TransceiverContext<A, B>,
    sleep_duration: Duration,
    callback: C,
) -> Result<TransceiverThread<DataAndAddr, ()>, io::Error>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
    C: Fn(Data, NetResponder) + Send + 'static,
{
    // Must be non-blocking so we can accept new connections within the same
    // thread as sending/receiving data
    listener.set_nonblocking(true)?;

    let (tx, rx) = mpsc::channel::<DataAndAddr>();

    let handle = thread::spawn(move || {
        let mut connections = HashMap::new();
        loop {
            // Process a new connection if we have one
            match listener.accept() {
                Ok((stream, addr)) => {
                    let (tx, rx) = mpsc::channel::<Data>();
                    connections.insert(addr, (stream, tx, rx));
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => (),
                // TODO: Handle errors
                Err(_e) => (),
            }

            // Attempt to send data on stream if there is any available
            match rx.try_recv() {
                Ok((data, addr)) => {
                    if let Some((_, tx, _)) = connections.get(&addr) {
                        // TODO: Handle errors
                        tx.send(data).unwrap();
                    }
                }
                Err(mpsc::TryRecvError::Empty) => (),
                // TODO: Handle errors
                Err(mpsc::TryRecvError::Disconnected) => panic!("Disconnected!"),
            }

            // Run through all streams
            for (_addr, (stream, tx, rx)) in connections.iter_mut() {
                let ns = NetResponder { tx: tx.clone() };

                // TODO: Handle errors
                net::tcp::stream::stream_process(stream, &mut ctx, rx, &ns, &callback).unwrap();
            }

            thread::sleep(sleep_duration);
        }
    });

    Ok(TransceiverThread { handle, tx })
}
