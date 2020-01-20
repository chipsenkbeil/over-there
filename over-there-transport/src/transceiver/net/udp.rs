use crate::transceiver::{
    receiver::{self, ReceiverError},
    transmitter::{self, TransmitterError},
    TransceiverContext,
};
use over_there_auth::{Signer, Verifier};
use over_there_crypto::{Decrypter, Encrypter};
use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};

pub struct UdpTransceiver<A, B>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
{
    pub socket: UdpSocket,
    ctx: Arc<RwLock<TransceiverContext<A, B>>>,
}

impl<A, B> UdpTransceiver<A, B>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
{
    pub fn new(socket: UdpSocket, ctx: TransceiverContext<A, B>) -> Self {
        Self {
            socket,
            ctx: Arc::new(RwLock::new(ctx)),
        }
    }

    pub fn send(&self, addr: SocketAddr, data: &[u8]) -> Result<(), TransmitterError> {
        send(&self.socket, addr, &mut self.ctx.write().unwrap(), data)
    }

    pub fn recv(&self) -> Result<(Option<Vec<u8>>, SocketAddr), ReceiverError> {
        recv(&self.socket, &mut self.ctx.write().unwrap())
    }

    pub fn spawn(
        &self,
        callback: impl Fn(Vec<u8>, SocketAddr) + Send + 'static,
    ) -> Result<JoinHandle<()>, io::Error> {
        spawn(self.socket.try_clone()?, Arc::clone(&self.ctx), callback)
    }
}

fn spawn<A, B>(
    socket: UdpSocket,
    ctx: Arc<RwLock<TransceiverContext<A, B>>>,
    callback: impl Fn(Vec<u8>, SocketAddr) + Send + 'static,
) -> Result<JoinHandle<()>, io::Error>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
{
    Ok(thread::spawn(move || loop {
        match recv(&socket, &mut ctx.write().unwrap()) {
            Ok((Some(data), addr)) => callback(data, addr),
            Ok((None, _addr)) => (),
            Err(_) => (),
        }
    }))
}

fn send<A, B>(
    socket: &UdpSocket,
    addr: SocketAddr,
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
        let size = socket.send_to(&data, addr)?;
        if size < data.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Only sent {} bytes out of {}", size, data.len()),
            ));
        }

        Ok(size)
    })
}

fn recv<A, B>(
    socket: &UdpSocket,
    ctx: &mut TransceiverContext<A, B>,
) -> Result<(Option<Vec<u8>>, SocketAddr), ReceiverError>
where
    A: Signer + Verifier,
    B: Encrypter + Decrypter,
{
    receiver::do_receive(From::from(ctx), |data| socket.recv_from(data))
}
