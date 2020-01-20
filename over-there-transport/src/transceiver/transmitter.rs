use crate::{
    disassembler::{self, DisassembleInfo, Disassembler},
    packet::PacketEncryption,
    transceiver::TransceiverContext,
};
use over_there_auth::{Signer, Verifier};
use over_there_crypto::{CryptError, Decrypter, Encrypter};
use over_there_derive::Error;
use rand::random;
use std::io::Error as IoError;

#[derive(Debug, Error)]
pub enum TransmitterError {
    EncodePacket(rmp_serde::encode::Error),
    DisassembleData(disassembler::DisassemblerError),
    EncryptData(CryptError),
    SendBytes(IoError),
}

pub(crate) struct TransmitterContext<'a, S, E>
where
    S: Signer,
    E: Encrypter,
{
    /// Maximum size allowed for a packet
    transmission_size: usize,

    /// Disassembler used to break up data into packets
    disassembler: &'a mut Disassembler,

    /// Performs authentication on data
    signer: &'a S,

    /// Performs encryption/decryption on data
    encrypter: &'a E,
}

impl<'a, A, B> From<&'a mut TransceiverContext<A, B>> for TransmitterContext<'a, A, B>
where
    A: Signer + Verifier,
    B: Encrypter + Decrypter,
{
    fn from(ctx: &'a mut TransceiverContext<A, B>) -> Self {
        Self {
            transmission_size: ctx.transmission_size,
            disassembler: &mut ctx.disassembler,
            signer: &ctx.authenticator,
            encrypter: &ctx.bicrypter,
        }
    }
}

pub(crate) fn do_send<'a, S, E, F>(
    ctx: TransmitterContext<'a, S, E>,
    data: &[u8],
    mut write: F,
) -> Result<(), TransmitterError>
where
    S: Signer,
    E: Encrypter,
    F: FnMut(&[u8]) -> Result<usize, IoError>,
{
    // Encrypt entire dataset before splitting as it will grow in size
    // and it's difficult to predict if we can stay under our transmission
    // limit if encrypting at the individual packet level
    let associated_data = ctx.encrypter.new_encrypt_associated_data();
    let encryption = PacketEncryption::from(associated_data.nonce().cloned());
    let data = ctx
        .encrypter
        .encrypt(data, &associated_data)
        .map_err(TransmitterError::EncryptData)?;

    // Produce a unique id used to group our packets
    let id: u32 = random();

    // Split data into multiple packets
    // NOTE: Must protect mutable access to disassembler, which caches
    //       computing the estimated packet sizes; if there is a way
    //       that we could do this faster (not need a cache), we could
    //       get rid of the locking and only need a reference
    let packets = ctx
        .disassembler
        .make_packets_from_data(DisassembleInfo {
            id,
            encryption,
            data: &data,
            desired_chunk_size: ctx.transmission_size,
            signer: ctx.signer,
        })
        .map_err(TransmitterError::DisassembleData)?;

    // For each packet, serialize and send to specific address
    for packet in packets.iter() {
        let packet_data = packet.to_vec().map_err(TransmitterError::EncodePacket)?;

        // TODO: Handle case where cannot send all bytes at once, which
        //       results in an invalid packet. Can we tack on a couple of
        //       bytes at the beginning to denote the total size of the
        //       serialized packet (in total) and read that before
        //       deserializing a packet?
        write(&packet_data).map_err(TransmitterError::SendBytes)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transceiver::TransceiverContext;
    use over_there_auth::NoopAuthenticator;
    use over_there_crypto::NoopBicrypter;
    use std::io::ErrorKind as IoErrorKind;
    use std::time::Duration;

    fn new_context(buffer_size: usize) -> TransceiverContext<NoopAuthenticator, NoopBicrypter> {
        TransceiverContext::new(
            buffer_size,
            Duration::from_secs(1),
            NoopAuthenticator,
            NoopBicrypter,
        )
    }

    #[test]
    fn do_send_should_fail_if_unable_to_convert_bytes_to_packets() {
        // Produce a transmitter with a "bytes per packet" that is too
        // low, causing the process to fail
        let mut ctx = new_context(0);
        let data = vec![1, 2, 3];

        match do_send(From::from(&mut ctx), &data, |_| Ok(data.len())) {
            Err(TransmitterError::DisassembleData(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn do_send_should_fail_if_fails_to_do_send_bytes() {
        let mut ctx = new_context(100);
        let data = vec![1, 2, 3];

        match do_send(From::from(&mut ctx), &data, |_| {
            Err(IoError::from(IoErrorKind::Other))
        }) {
            Err(TransmitterError::SendBytes(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn do_send_should_return_okay_if_successfully_sent_data() {
        let mut ctx = new_context(100);
        let data = vec![1, 2, 3];

        let result = do_send(From::from(&mut ctx), &data, |_| Ok(data.len()));
        assert_eq!(result.is_ok(), true);
    }

    #[cfg(test)]
    mod crypt {
        use super::*;
        use over_there_crypto::{AssociatedData, CryptError, Decrypter, Encrypter};

        struct BadEncrypter;
        impl Encrypter for BadEncrypter {
            fn encrypt(&self, _: &[u8], _: &AssociatedData) -> Result<Vec<u8>, CryptError> {
                Err(CryptError::EncryptFailed(From::from("Some error")))
            }

            fn new_encrypt_associated_data(&self) -> AssociatedData {
                AssociatedData::None
            }
        }
        impl Decrypter for BadEncrypter {
            fn decrypt(&self, _: &[u8], _: &AssociatedData) -> Result<Vec<u8>, CryptError> {
                Err(CryptError::DecryptFailed(From::from("Some error")))
            }
        }

        fn new_context(buffer_size: usize) -> TransceiverContext<NoopAuthenticator, BadEncrypter> {
            TransceiverContext::new(
                buffer_size,
                Duration::from_secs(1),
                NoopAuthenticator,
                BadEncrypter,
            )
        }

        #[test]
        fn do_send_should_fail_if_unable_to_encrypt_data() {
            let mut ctx = new_context(100);
            let data = vec![1, 2, 3];

            match do_send(From::from(&mut ctx), &data, |_| Ok(data.len())) {
                Err(super::TransmitterError::EncryptData(_)) => (),
                x => panic!("Unexpected result: {:?}", x),
            }
        }
    }
}
