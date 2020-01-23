use crate::transceiver::{
    net::NetResponder,
    receiver::{self, ReceiverError},
    transmitter::{self, TransmitterError},
    Responder, ResponderError, TransceiverContext, TransceiverThread,
};
use over_there_auth::{Signer, Verifier};
use over_there_crypto::{Decrypter, Encrypter};
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread::{self};
use std::time::Duration;

#[derive(Clone)]
pub struct TcpNetResponder {
    tx: mpsc::Sender<Vec<u8>>,
    addr: SocketAddr,
}

impl Responder for TcpNetResponder {
    fn send(&self, data: &[u8]) -> Result<(), ResponderError> {
        self.tx
            .send(data.to_vec())
            .map_err(|_| ResponderError::NoLongerAvailable)
    }
}

impl NetResponder for TcpNetResponder {
    fn addr(&self) -> SocketAddr {
        self.addr
    }
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

    pub fn spawn<C>(
        self,
        sleep_duration: Duration,
        callback: C,
    ) -> io::Result<TransceiverThread<(), (Vec<u8>, SocketAddr)>>
    where
        C: Fn(Vec<u8>, TcpNetResponder) + Send + 'static,
    {
        listener_spawn(self.listener, self.ctx, sleep_duration, callback)
    }
}

pub struct TcpStreamTransceiver<A, B>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
{
    pub stream: TcpStream,
    ctx: TransceiverContext<A, B>,
}

impl<A, B> TcpStreamTransceiver<A, B>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
{
    pub fn new(stream: TcpStream, ctx: TransceiverContext<A, B>) -> Self {
        Self { stream, ctx }
    }

    pub fn spawn<C>(
        self,
        sleep_duration: Duration,
        callback: C,
    ) -> io::Result<TransceiverThread<(), Vec<u8>>>
    where
        C: Fn(Vec<u8>, TcpNetResponder) + Send + 'static,
    {
        stream_spawn(self.stream, self.ctx, sleep_duration, callback)
    }

    pub fn send(&mut self, data: &[u8]) -> Result<(), TransmitterError> {
        stream_send(&mut self.stream, &mut self.ctx, data)
    }

    pub fn recv(&mut self) -> Result<Option<Vec<u8>>, ReceiverError> {
        stream_recv(&mut self.stream, &mut self.ctx)
    }
}

fn listener_spawn<A, B, C>(
    listener: TcpListener,
    mut ctx: TransceiverContext<A, B>,
    sleep_duration: Duration,
    callback: C,
) -> Result<TransceiverThread<(), (Vec<u8>, SocketAddr)>, io::Error>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
    C: Fn(Vec<u8>, TcpNetResponder) + Send + 'static,
{
    // Must be non-blocking so we can accept new connections within the same
    // thread as sending/receiving data
    listener.set_nonblocking(true)?;

    let (tx, rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>();

    let handle = thread::spawn(move || {
        let mut connections = HashMap::new();
        loop {
            // Process a new connection if we have one
            match listener.accept() {
                Ok((stream, addr)) => {
                    let (tx, rx) = mpsc::channel::<Vec<u8>>();
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
            for (addr, (stream, tx, rx)) in connections.iter_mut() {
                let tns = TcpNetResponder {
                    tx: tx.clone(),
                    addr: *addr,
                };

                // TODO: Handle errors
                stream_process(stream, &mut ctx, rx, &tns, &callback).unwrap();
            }

            thread::sleep(sleep_duration);
        }
    });

    Ok(TransceiverThread { handle, tx })
}

fn stream_spawn<A, B, C>(
    mut stream: TcpStream,
    mut ctx: TransceiverContext<A, B>,
    sleep_duration: Duration,
    callback: C,
) -> Result<TransceiverThread<(), Vec<u8>>, io::Error>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
    C: Fn(Vec<u8>, TcpNetResponder) + Send + 'static,
{
    let addr = stream.peer_addr()?;
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let thread_tx = tx.clone();

    let handle = thread::spawn(move || {
        let tns = TcpNetResponder {
            tx: thread_tx,
            addr,
        };
        loop {
            // TODO: Handle errors
            stream_process(&mut stream, &mut ctx, &rx, &tns, &callback).unwrap();

            thread::sleep(sleep_duration);
        }
    });

    Ok(TransceiverThread { handle, tx })
}

fn stream_process<A, B, C>(
    stream: &mut TcpStream,
    ctx: &mut TransceiverContext<A, B>,
    send_rx: &mpsc::Receiver<Vec<u8>>,
    tns: &TcpNetResponder,
    callback: &C,
) -> Result<(), io::Error>
where
    A: Signer + Verifier,
    B: Encrypter + Decrypter,
    C: Fn(Vec<u8>, TcpNetResponder) + Send + 'static,
{
    // Attempt to send data on socket if there is any available
    match send_rx.try_recv() {
        Ok(data) => stream_send(stream, ctx, &data).unwrap(),
        Err(mpsc::TryRecvError::Empty) => (),
        // TODO: Handle errors
        Err(mpsc::TryRecvError::Disconnected) => panic!("Disconnected!"),
    }

    match stream_recv(stream, ctx) {
        Ok(Some(data)) => {
            callback(data, tns.clone());
            Ok(())
        }
        Ok(None) => Ok(()),
        // TODO: Handle errors
        Err(x) => panic!("Unexpected error: {:?}", x),
    }
}

/// Helper method to send data using the underlying stream
fn stream_send<A, B>(
    stream: &mut TcpStream,
    ctx: &mut TransceiverContext<A, B>,
    data: &[u8],
) -> Result<(), TransmitterError>
where
    A: Signer + Verifier,
    B: Encrypter + Decrypter,
{
    transmitter::do_send(From::from(ctx), data, |data| {
        // TODO: Support sending remaining bytes in loop? Would need to
        //       support collecting bytes for a packet in multiple receives,
        //       which means we'd need a start and stop indicator of some
        //       kind that is a single byte. Seems too complicated, so
        //       easier to fail and give a reason if we don't send all
        //       of the bytes in one go. It's one of the reasons we made
        //       packets of a guaranteed max size.
        let size = stream.write(&data)?;
        if size < data.len() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Only sent {} bytes out of {}", size, data.len()),
            ));
        }

        Ok(size)
    })
}

/// Helper method to receive data using the underlying stream
fn stream_recv<A, B>(
    stream: &mut TcpStream,
    ctx: &mut TransceiverContext<A, B>,
) -> Result<Option<Vec<u8>>, ReceiverError>
where
    A: Signer + Verifier,
    B: Encrypter + Decrypter,
{
    receiver::do_receive(From::from(ctx), |data| stream.read(data).map(|s| (s, ())))
        .map(|o1| o1.map(|o2| o2.0))
}
