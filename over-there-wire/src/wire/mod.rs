mod input;
mod output;
mod packet;

use over_there_derive::Error;
use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::time::Duration;

// Export errors
pub use input::assembler::AssemblerError;
pub use input::{InputProcessor, InputProcessorError};
pub use output::disassembler::DisassemblerError;
pub use output::{OutputProcessor, OutputProcessorError};

// Re-export the auth and crypto interfaces
pub use over_there_auth::{Signer, Verifier};
pub use over_there_crypto::{Decrypter, Encrypter};

#[derive(Debug, Error)]
pub enum InboundWireError {
    IO(io::Error),
    InputProcessor(InputProcessorError),
}

#[derive(Debug, Error)]
pub enum OutboundWireError {
    IO(io::Error),
    OutputProcessor(OutputProcessorError),

    /// When fail to send all bytes out together on the wire
    IncompleteSend,
}

/// Wire for inbound communication
#[derive(Debug, Clone)]
pub struct InboundWire<V, D>
where
    V: Verifier + Send + 'static,
    D: Decrypter + Send + 'static,
{
    /// Maximum size of expected data
    transmission_size: usize,

    /// Processes input coming into the wire
    input_processor: InputProcessor<V, D>,
}

impl<V, D> InboundWire<V, D>
where
    V: Verifier + Send + 'static,
    D: Decrypter + Send + 'static,
{
    pub fn new(transmission_size: usize, packet_ttl: Duration, verifier: V, decrypter: D) -> Self {
        let input_processor = InputProcessor::new(packet_ttl, verifier, decrypter);
        Self {
            transmission_size,
            input_processor,
        }
    }

    pub fn transmission_size(&self) -> usize {
        self.transmission_size
    }

    /// CHIP CHIP CHIP
    ///
    /// UdpSocket.split creates Arc<UdpSocket> and provides one to each
    /// of the Recv and Send halves
    ///
    /// TcpStream.split creates &'a TcpStream and provides one to each of
    /// the Read and Write halves
    ///
    /// Seems like easiest approach is to create Arc of stream and socket,
    /// try_clone does not work with mio::Event per https://github.com/tokio-rs/tokio/pull/1308#issuecomment-513337003
    ///
    /// Tokio used to have try_clone, but was deprecated via
    /// https://github.com/tokio-rs/tokio/pull/824
    ///
    /// <<<>>>
    ///
    /// Explanation of UdpSocket split versus TcpStream split here:
    /// https://github.com/tokio-rs/tokio/pull/1226#issuecomment-538033875
    ///
    /// Maybe best path is to create some wrapper structure that will contain
    /// the TcpStream and perform the split
    ///
    /// <<<>>>
    ///
    /// It might make more sense to use the select! macro instead, which
    /// can then run on a single thread and not have to split
    ///
    /// VV DO THIS vvv
    ///
    /// https://docs.rs/tokio/0.2.11/tokio/macro.select.html
    ///
    /// ^^^ DO THIS ^^^

    /// Receives data synchronously using the provided function
    /// If None returned for data, indicates that bytes were received but
    /// the full message has yet to be collected
    pub fn recv<F>(&mut self, f: F) -> Result<(Option<Vec<u8>>, SocketAddr), InboundWireError>
    where
        F: FnOnce(&mut [u8]) -> io::Result<(usize, SocketAddr)>,
    {
        let mut buf = vec![0; self.transmission_size].into_boxed_slice();
        let (size, addr) = f(&mut buf).map_err(InboundWireError::IO)?;
        let data = self.process(&buf[..size])?;
        Ok((data, addr))
    }

    /// Receives data asynchronously using the provided function
    /// If None returned for data, indicates that bytes were received but
    /// the full message has yet to be collected
    pub async fn async_recv<F, R>(
        &mut self,
        f: F,
    ) -> Result<(Option<Vec<u8>>, SocketAddr), InboundWireError>
    where
        F: FnOnce(&mut [u8]) -> R,
        R: Future<Output = io::Result<(usize, SocketAddr)>>,
    {
        let mut buf = vec![0; self.transmission_size].into_boxed_slice();
        let (size, addr) = f(&mut buf).await.map_err(InboundWireError::IO)?;
        let data = self.process(&buf[..size])?;
        Ok((data, addr))
    }

    /// Processes received data
    #[inline]
    pub fn process(&mut self, buf: &[u8]) -> Result<Option<Vec<u8>>, InboundWireError> {
        self.input_processor
            .process(buf)
            .map_err(InboundWireError::InputProcessor)
    }
}

/// Wire for outbound communication
#[derive(Debug, Clone)]
pub struct OutboundWire<S, E>
where
    S: Signer + Send + 'static,
    E: Encrypter + Send + 'static,
{
    /// Processes output leaving on the wire
    output_processor: OutputProcessor<S, E>,
}

impl<S, E> OutboundWire<S, E>
where
    S: Signer + Send + 'static,
    E: Encrypter + Send + 'static,
{
    pub fn new(transmission_size: usize, signer: S, encrypter: E) -> Self {
        let output_processor = OutputProcessor::new(transmission_size, signer, encrypter);
        Self { output_processor }
    }

    /// Sends data in buf synchronously using the provided function
    pub fn send<F>(&mut self, buf: &[u8], mut f: F) -> Result<(), OutboundWireError>
    where
        F: FnMut(&[u8]) -> io::Result<usize>,
    {
        let data = self.process(buf)?;

        for packet_bytes in data.iter() {
            let size = f(packet_bytes).map_err(OutboundWireError::IO)?;
            if size < packet_bytes.len() {
                return Err(OutboundWireError::IncompleteSend);
            }
        }

        Ok(())
    }

    /// Sends data in buf asynchronously using the provided function
    pub async fn async_send<F, R>(&mut self, buf: &[u8], mut f: F) -> Result<(), OutboundWireError>
    where
        F: FnMut(&[u8]) -> R,
        R: Future<Output = io::Result<usize>>,
    {
        let data = self.process(buf)?;

        for packet_bytes in data.iter() {
            let size = f(packet_bytes).await.map_err(OutboundWireError::IO)?;
            if size < packet_bytes.len() {
                return Err(OutboundWireError::IncompleteSend);
            }
        }

        Ok(())
    }

    /// Processes outgoing data
    #[inline]
    pub fn process(&mut self, buf: &[u8]) -> Result<Vec<Vec<u8>>, OutboundWireError> {
        self.output_processor
            .process(buf)
            .map_err(OutboundWireError::OutputProcessor)
    }
}
