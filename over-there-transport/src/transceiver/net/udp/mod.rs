pub mod stream;

use crate::transceiver::{
    net::{AddrNetResponder, Data, DataAndAddr, NetListener},
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
pub enum UdpTransceiverError {
    SendError(TransmitterError),
    RecvError(ReceiverError),
    Disconnected,
}

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
    type Error = UdpTransceiverError;
    type Responder = AddrNetResponder;

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
}

fn spawn<A, B, C, D>(
    socket: UdpSocket,
    mut ctx: TransceiverContext<A, B>,
    sleep_duration: Duration,
    callback: C,
    err_callback: D,
) -> TransceiverThread<DataAndAddr, ()>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
    C: Fn(Data, AddrNetResponder) + Send + 'static,
    D: Fn(UdpTransceiverError) -> bool + Send + 'static,
{
    let (tx, rx) = mpsc::channel::<DataAndAddr>();
    let thread_tx = tx.clone();
    let handle = thread::Builder::new()
        .name(String::from("udp-transceiver"))
        .spawn(move || loop {
            if let Err(e) = process(&socket, &mut ctx, &rx, &thread_tx, &callback) {
                if !err_callback(e) {
                    break;
                }
            }
            thread::sleep(sleep_duration);
        })
        .expect("failed to spawn udp transceiver thread");
    TransceiverThread { handle, tx }
}

fn process<A, B, C>(
    socket: &UdpSocket,
    ctx: &mut TransceiverContext<A, B>,
    send_rx: &mpsc::Receiver<DataAndAddr>,
    tx: &mpsc::Sender<DataAndAddr>,
    callback: &C,
) -> Result<(), UdpTransceiverError>
where
    A: Signer + Verifier,
    B: Encrypter + Decrypter,
    C: Fn(Data, AddrNetResponder) + Send + 'static,
{
    // Attempt to send data on socket if there is any available
    match send_rx.try_recv() {
        Ok((data, addr)) => {
            if let Err(e) = send(socket, addr, ctx, &data) {
                return Err(UdpTransceiverError::SendError(e));
            }
        }
        Err(mpsc::TryRecvError::Empty) => (),
        Err(mpsc::TryRecvError::Disconnected) => return Err(UdpTransceiverError::Disconnected),
    }

    // Attempt to get new data and pass it along
    match recv(socket, ctx) {
        Ok(Some((data, addr))) => {
            callback(
                data,
                AddrNetResponder {
                    tx: tx.clone(),
                    addr,
                },
            );
            Ok(())
        }
        Ok(None) => Ok(()),
        Err(x) => Err(UdpTransceiverError::RecvError(x)),
    }
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
