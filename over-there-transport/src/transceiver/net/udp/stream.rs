use crate::transceiver::{
    net::{Data, NetResponder, NetStream},
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

pub struct UdpStreamTransceiver<A, B>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
{
    pub socket: UdpSocket,
    pub addr: SocketAddr,
    pub(super) ctx: TransceiverContext<A, B>,
}

impl<A, B> NetStream for UdpStreamTransceiver<A, B>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
{
    fn spawn<C>(
        self,
        sleep_duration: Duration,
        callback: C,
    ) -> io::Result<TransceiverThread<Data, ()>>
    where
        C: Fn(Data, NetResponder) + Send + 'static,
    {
        let thread = spawn(self.socket, self.ctx, sleep_duration, callback);
        Ok(thread)
    }

    fn send(&mut self, data: &[u8]) -> Result<(), TransmitterError> {
        send(&self.socket, &mut self.ctx, data)
    }

    fn recv(&mut self) -> Result<Option<Data>, ReceiverError> {
        recv(&self.socket, &mut self.ctx)
    }
}

fn spawn<A, B, C>(
    socket: UdpSocket,
    mut ctx: TransceiverContext<A, B>,
    sleep_duration: Duration,
    callback: C,
) -> TransceiverThread<Data, ()>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
    C: Fn(Data, NetResponder) + Send + 'static,
{
    let (tx, rx) = mpsc::channel::<Data>();
    let thread_tx = tx.clone();
    let handle = thread::spawn(move || {
        loop {
            // Attempt to send data on socket if there is any available
            match rx.try_recv() {
                Ok(data) => send(&socket, &mut ctx, &data).unwrap(),
                Err(mpsc::TryRecvError::Empty) => (),
                Err(x) => panic!("Unexpected error: {:?}", x),
            }

            // Attempt to get new data and pass it along
            match recv(&socket, &mut ctx) {
                Ok(Some(data)) => callback(
                    data,
                    NetResponder {
                        tx: thread_tx.clone(),
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
        let size = socket.send(&data)?;
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
) -> Result<Option<Data>, ReceiverError>
where
    A: Signer + Verifier,
    B: Encrypter + Decrypter,
{
    receiver::do_receive(From::from(ctx), |data| {
        socket.recv(data).map(|size| (size, ()))
    })
    .map(|r| r.map(|d| d.0))
}
