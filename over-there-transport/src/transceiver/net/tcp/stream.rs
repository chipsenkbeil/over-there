use crate::transceiver::{
    net::{Data, NetResponder, NetStream},
    receiver::{self, ReceiverError},
    transmitter::{self, TransmitterError},
    TransceiverContext, TransceiverThread,
};
use over_there_auth::{Signer, Verifier};
use over_there_crypto::{Decrypter, Encrypter};
use over_there_derive::Error;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::sync::mpsc;
use std::thread::{self};
use std::time::Duration;

#[derive(Debug, Error)]
pub enum TcpStreamTransceiverError {
    SendError(TransmitterError),
    RecvError(ReceiverError),
    Disconnected,
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
}

impl<A, B> NetStream for TcpStreamTransceiver<A, B>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
{
    type Error = TcpStreamTransceiverError;

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
        // NOTE: Stream MUST have a read timeout otherwise it will block indefinitely
        self.stream
            .set_read_timeout(Some(Duration::from_millis(1)))?;

        stream_spawn(
            self.stream,
            self.ctx,
            sleep_duration,
            callback,
            err_callback,
        )
    }

    fn send(&mut self, data: &[u8]) -> Result<(), TransmitterError> {
        stream_send(&mut self.stream, &mut self.ctx, data)
    }

    fn recv(&mut self) -> Result<Option<Data>, ReceiverError> {
        stream_recv(&mut self.stream, &mut self.ctx)
    }
}

fn stream_spawn<A, B, C, D>(
    mut stream: TcpStream,
    mut ctx: TransceiverContext<A, B>,
    sleep_duration: Duration,
    callback: C,
    err_callback: D,
) -> Result<TransceiverThread<Data, ()>, io::Error>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
    C: Fn(Data, NetResponder) + Send + 'static,
    D: Fn(TcpStreamTransceiverError) -> bool + Send + 'static,
{
    let (tx, rx) = mpsc::channel::<Data>();
    let thread_tx = tx.clone();

    let handle = thread::spawn(move || {
        let tns = NetResponder { tx: thread_tx };
        loop {
            if let Err(e) = stream_process(&mut stream, &mut ctx, &rx, &tns, &callback) {
                if !err_callback(e) {
                    break;
                }
            }

            thread::sleep(sleep_duration);
        }
    });

    Ok(TransceiverThread { handle, tx })
}

pub(super) fn stream_process<A, B, C>(
    stream: &mut TcpStream,
    ctx: &mut TransceiverContext<A, B>,
    send_rx: &mpsc::Receiver<Data>,
    ns: &NetResponder,
    callback: &C,
) -> Result<(), TcpStreamTransceiverError>
where
    A: Signer + Verifier,
    B: Encrypter + Decrypter,
    C: Fn(Data, NetResponder) + Send + 'static,
{
    // Attempt to send data on socket if there is any available
    match send_rx.try_recv() {
        Ok(data) => {
            if let Err(e) = stream_send(stream, ctx, &data) {
                return Err(TcpStreamTransceiverError::SendError(e));
            }
        }
        Err(mpsc::TryRecvError::Empty) => (),
        Err(mpsc::TryRecvError::Disconnected) => {
            return Err(TcpStreamTransceiverError::Disconnected)
        }
    }

    match stream_recv(stream, ctx) {
        Ok(Some(data)) => {
            callback(data, ns.clone());
            Ok(())
        }
        Ok(None) => Ok(()),
        Err(x) => Err(TcpStreamTransceiverError::RecvError(x)),
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
) -> Result<Option<Data>, ReceiverError>
where
    A: Signer + Verifier,
    B: Encrypter + Decrypter,
{
    receiver::do_receive(From::from(ctx), |data| stream.read(data).map(|s| (s, ())))
        .map(|o1| o1.map(|o2| o2.0))
}
