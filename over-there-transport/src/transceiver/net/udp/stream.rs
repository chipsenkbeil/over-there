use crate::transceiver::{
    net::{Data, NetResponder, NetStream},
    receiver::{self, ReceiverError},
    transmitter::{self, TransmitterError},
    TransceiverContext, TransceiverThread,
};
use over_there_auth::{Signer, Verifier};
use over_there_crypto::{Decrypter, Encrypter};
use over_there_derive::Error;
use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Debug, Error)]
pub enum UdpStreamTransceiverError {
    SendError(TransmitterError),
    RecvError(ReceiverError),
    Disconnected,
}

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
    type Error = UdpStreamTransceiverError;

    fn spawn<C, D>(
        self,
        sleep_duration: Duration,
        callback: C,
        err_callback: D,
    ) -> io::Result<TransceiverThread<Data, ()>>
    where
        C: Fn(Data, NetResponder) + Send + 'static,
        D: Fn(Self::Error) -> bool + Send + 'static,
    {
        // NOTE: Socket MUST have a read timeout otherwise it will block indefinitely
        self.socket
            .set_read_timeout(Some(Duration::from_millis(1)))?;

        Ok(spawn(
            self.socket,
            self.ctx,
            sleep_duration,
            callback,
            err_callback,
        ))
    }

    fn send(&mut self, data: &[u8]) -> Result<(), TransmitterError> {
        send(&self.socket, &mut self.ctx, data)
    }

    fn recv(&mut self) -> Result<Option<Data>, ReceiverError> {
        recv(&self.socket, &mut self.ctx)
    }
}

fn spawn<A, B, C, D>(
    socket: UdpSocket,
    mut ctx: TransceiverContext<A, B>,
    sleep_duration: Duration,
    callback: C,
    err_callback: D,
) -> TransceiverThread<Data, ()>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
    C: Fn(Data, NetResponder) + Send + 'static,
    D: Fn(UdpStreamTransceiverError) -> bool + Send + 'static,
{
    let (tx, rx) = mpsc::channel::<Data>();
    let ns = NetResponder { tx: tx.clone() };
    let handle = thread::Builder::new()
        .name(String::from("udp-transceiver-stream"))
        .spawn(move || loop {
            if let Err(e) = process(&socket, &mut ctx, &rx, &ns, &callback) {
                if !err_callback(e) {
                    break;
                }
            }
            thread::sleep(sleep_duration);
        })
        .expect("failed to spawn udp transceiver stream thread");
    TransceiverThread { handle, tx }
}

fn process<A, B, C>(
    socket: &UdpSocket,
    ctx: &mut TransceiverContext<A, B>,
    send_rx: &mpsc::Receiver<Data>,
    ns: &NetResponder,
    callback: &C,
) -> Result<(), UdpStreamTransceiverError>
where
    A: Signer + Verifier,
    B: Encrypter + Decrypter,
    C: Fn(Data, NetResponder) + Send + 'static,
{
    // Attempt to send data on socket if there is any available
    match send_rx.try_recv() {
        Ok(data) => {
            if let Err(e) = send(socket, ctx, &data) {
                return Err(UdpStreamTransceiverError::SendError(e));
            }
        }
        Err(mpsc::TryRecvError::Empty) => (),
        Err(mpsc::TryRecvError::Disconnected) => {
            return Err(UdpStreamTransceiverError::Disconnected)
        }
    }

    // Attempt to get new data and pass it along
    match recv(socket, ctx) {
        Ok(Some(data)) => {
            callback(data, ns.clone());
            Ok(())
        }
        Ok(None) => Ok(()),
        Err(x) => Err(UdpStreamTransceiverError::RecvError(x)),
    }
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
