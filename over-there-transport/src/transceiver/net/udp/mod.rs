pub mod stream;

use crate::transceiver::{
    net::{AddrNetResponder, Data, DataAndAddr, NetListener},
    receiver::{self, ReceiverError},
    transmitter::{self, TransmitterError},
    TransceiverContext, TransceiverThread,
};
use over_there_auth::{Signer, Verifier};
use over_there_crypto::{Decrypter, Encrypter};
use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub struct UdpTransceiver<A, B>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
{
    pub socket: UdpSocket,
    ctx: TransceiverContext<A, B>,
}

impl<A, B> UdpTransceiver<A, B>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
{
    pub fn new(socket: UdpSocket, ctx: TransceiverContext<A, B>) -> Self {
        Self { socket, ctx }
    }

    /// Attempts to connect this transceiver directly to a remote address,
    /// filtering all other communication not originating from that address
    pub fn connect(self, addr: SocketAddr) -> io::Result<stream::UdpStreamTransceiver<A, B>> {
        self.socket.connect(addr)?;
        Ok(stream::UdpStreamTransceiver {
            socket: self.socket,
            addr,
            ctx: self.ctx,
        })
    }

    pub fn send(&mut self, addr: SocketAddr, data: &[u8]) -> Result<(), TransmitterError> {
        send(&self.socket, addr, &mut self.ctx, data)
    }

    pub fn recv(&mut self) -> Result<Option<DataAndAddr>, ReceiverError> {
        recv(&self.socket, &mut self.ctx)
    }
}

impl<A, B> NetListener for UdpTransceiver<A, B>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
{
    type Responder = AddrNetResponder;

    fn spawn<C>(
        self,
        sleep_duration: Duration,
        callback: C,
    ) -> io::Result<TransceiverThread<DataAndAddr, ()>>
    where
        C: Fn(Data, Self::Responder) + Send + 'static,
    {
        Ok(spawn(self.socket, self.ctx, sleep_duration, callback))
    }
}

fn spawn<A, B, C>(
    socket: UdpSocket,
    mut ctx: TransceiverContext<A, B>,
    sleep_duration: Duration,
    callback: C,
) -> TransceiverThread<DataAndAddr, ()>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
    C: Fn(Data, AddrNetResponder) + Send + 'static,
{
    let (tx, rx) = mpsc::channel::<DataAndAddr>();
    let thread_tx = tx.clone();
    let handle = thread::spawn(move || {
        loop {
            // Attempt to send data on socket if there is any available
            match rx.try_recv() {
                Ok((data, addr)) => send(&socket, addr, &mut ctx, &data).unwrap(),
                Err(mpsc::TryRecvError::Empty) => (),
                Err(x) => panic!("Unexpected error: {:?}", x),
            }

            // Attempt to get new data and pass it along
            match recv(&socket, &mut ctx) {
                Ok(Some((data, addr))) => callback(
                    data,
                    AddrNetResponder {
                        tx: thread_tx.clone(),
                        addr,
                    },
                ),
                Ok(None) => (),

                // TODO: Handle errors
                Err(_) => (),
            }

            thread::sleep(sleep_duration);
        }
    });
    TransceiverThread { handle, tx }
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
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Only sent {} bytes out of {}", size, data.len()),
            ));
        }

        Ok(size)
    })
}

fn recv<A, B>(
    socket: &UdpSocket,
    ctx: &mut TransceiverContext<A, B>,
) -> Result<Option<DataAndAddr>, ReceiverError>
where
    A: Signer + Verifier,
    B: Encrypter + Decrypter,
{
    receiver::do_receive(From::from(ctx), |data| socket.recv_from(data))
}
