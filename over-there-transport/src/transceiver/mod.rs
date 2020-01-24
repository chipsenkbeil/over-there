pub mod net;
pub mod receiver;
pub mod transmitter;

use crate::assembler::Assembler;
use crate::disassembler::Disassembler;
use over_there_auth::{Signer, Verifier};
use over_there_crypto::{Decrypter, Encrypter};
use over_there_derive::Error;
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;

#[derive(Debug, Error)]
pub enum ResponderError {
    /// Indicates that the destination is no longer available for a response
    NoLongerAvailable,
}

pub trait Responder: Clone {
    fn send(&self, data: &[u8]) -> Result<(), ResponderError>;
}

#[derive(Debug, Error)]
pub enum TransceiverThreadError {
    FailedToSend,
    FailedToJoin,
}

pub struct TransceiverThread<T, R> {
    handle: JoinHandle<R>,
    tx: mpsc::Sender<T>,
}

impl<T, R> TransceiverThread<T, R> {
    pub fn send(&self, data: T) -> Result<(), TransceiverThreadError> {
        self.tx
            .send(data)
            .map_err(|_| TransceiverThreadError::FailedToSend)
    }

    pub fn join(self) -> Result<R, TransceiverThreadError> {
        self.handle
            .join()
            .map_err(|_| TransceiverThreadError::FailedToJoin)
    }
}

pub struct TransceiverContext<A, B>
where
    A: Signer + Verifier,
    B: Encrypter + Decrypter,
{
    /// Buffer to contain bytes for temporary storage
    /// NOTE: Heap allocation until we can control array size with const generics
    buffer: Box<[u8]>,

    /// Maximum size at which to send data
    transmission_size: usize,

    /// Assembler used to gather packets together
    assembler: Assembler,

    /// Disassembler used to break up data into packets
    disassembler: Disassembler,

    /// Performs signing/verification on data
    authenticator: A,

    /// Performs encryption/decryption on data
    bicrypter: B,
}

impl<A, B> TransceiverContext<A, B>
where
    A: Signer + Verifier,
    B: Encrypter + Decrypter,
{
    pub const DEFAULT_TTL_IN_SECS: u64 = Assembler::DEFAULT_TTL_IN_SECS;

    pub fn new(
        transmission_size: usize,
        packet_ttl: Duration,
        authenticator: A,
        bicrypter: B,
    ) -> Self {
        Self {
            buffer: vec![0; transmission_size].into_boxed_slice(),
            transmission_size,
            assembler: Assembler::new(packet_ttl),
            disassembler: Disassembler::default(),
            authenticator,
            bicrypter,
        }
    }
}
