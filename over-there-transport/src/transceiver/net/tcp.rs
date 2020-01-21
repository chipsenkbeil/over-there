use crate::transceiver::{
    net::NetSend,
    receiver::{self, ReceiverError},
    transmitter::{self, TransmitterError},
    TransceiverContext,
};
use over_there_auth::{Signer, Verifier};
use over_there_crypto::{Decrypter, Encrypter};
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::{mpsc, Arc, RwLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub struct TcpNetSend {
    tx: mpsc::Sender<Vec<u8>>,
    addr: SocketAddr,
}

impl NetSend for TcpNetSend {
    type TSendData = Vec<u8>;

    fn send(&self, data: &[u8]) -> Result<(), mpsc::SendError<Self::TSendData>> {
        self.tx.send(data.to_vec())
    }

    fn addr(&self) -> SocketAddr {
        self.addr
    }
}

impl Clone for TcpNetSend {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            addr: self.addr,
        }
    }
}

pub struct TcpStreamTransceiver<A, B>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
{
    pub stream: TcpStream,
    ctx: Arc<RwLock<TransceiverContext<A, B>>>,
}

impl<A, B> TcpStreamTransceiver<A, B>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
{
    pub fn new(stream: TcpStream, ctx: TransceiverContext<A, B>) -> Self {
        Self {
            stream,
            ctx: Arc::new(RwLock::new(ctx)),
        }
    }

    pub fn send(&mut self, data: &[u8]) -> Result<(), TransmitterError> {
        stream_send(&mut self.stream, &mut self.ctx.write().unwrap(), data)
    }

    pub fn recv(&mut self) -> Result<Option<Vec<u8>>, ReceiverError> {
        stream_recv(&mut self.stream, &mut self.ctx.write().unwrap())
    }

    pub fn spawn(
        &self,
        sleep_duration: Duration,
        callback: impl Fn(Vec<u8>, TcpNetSend) + Send + 'static,
    ) -> Result<JoinHandle<()>, io::Error> {
        stream_spawn(
            self.stream.try_clone()?,
            Arc::clone(&self.ctx),
            sleep_duration,
            callback,
        )
    }
}

fn stream_spawn<A, B>(
    mut stream: TcpStream,
    ctx: Arc<RwLock<TransceiverContext<A, B>>>,
    sleep_duration: Duration,
    callback: impl Fn(Vec<u8>, TcpNetSend) + Send + 'static,
) -> Result<JoinHandle<()>, io::Error>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
{
    let addr = stream.local_addr()?;
    Ok(thread::spawn(move || {
        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        loop {
            let mut ctx_mut = ctx.write().unwrap();

            // Attempt to send data on socket if there is any available
            // TODO: Handle non-timeout errors
            if let Ok(data) = rx.recv_timeout(Duration::new(0, 0)) {
                // TODO: Handle errors
                stream_send(&mut stream, &mut ctx_mut, &data).unwrap();
            }

            match stream_recv(&mut stream, &mut ctx_mut) {
                Ok(Some(data)) => callback(
                    data,
                    TcpNetSend {
                        tx: tx.clone(),
                        addr,
                    },
                ),
                Ok(None) => (),
                Err(_) => (),
            }
            thread::sleep(sleep_duration);
        }
    }))
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
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
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
    receiver::do_receive(From::from(ctx), |data| stream.read(data).map(|s| (s, ()))).map(|r| r.0)
}
