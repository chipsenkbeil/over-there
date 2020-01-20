use crate::transceiver::{
    receiver::{self, ReceiverError},
    transmitter::{self, TransmitterError},
    TransceiverContext,
};
use over_there_auth::{Signer, Verifier};
use over_there_crypto::{Decrypter, Encrypter};
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};

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
        callback: impl Fn(Vec<u8>) + Send + 'static,
    ) -> Result<JoinHandle<()>, io::Error> {
        stream_spawn(self.stream.try_clone()?, Arc::clone(&self.ctx), callback)
    }
}

fn stream_spawn<A, B>(
    mut stream: TcpStream,
    ctx: Arc<RwLock<TransceiverContext<A, B>>>,
    callback: impl Fn(Vec<u8>) + Send + 'static,
) -> Result<JoinHandle<()>, io::Error>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
{
    Ok(thread::spawn(move || loop {
        match stream_recv(&mut stream, &mut ctx.write().unwrap()) {
            Ok(Some(data)) => callback(data),
            Ok(None) => (),
            Err(_) => (),
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
