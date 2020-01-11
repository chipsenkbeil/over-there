use crate::disassembler::{self, DisassembleInfo, Disassembler};
use over_there_auth::Signer;
use over_there_crypto::{CryptError, Encrypter};
use over_there_derive::Error;
use rand::random;
use std::io::Error as IoError;
use std::sync::RwLock;

#[derive(Debug, Error)]
pub enum TransmitterError {
    EncodePacket(rmp_serde::encode::Error),
    DisassembleData(disassembler::DisassemblerError),
    EncryptData(CryptError),
    SendBytes(IoError),
}

/// Not thread-safe; should only be used in one thread
pub struct Transmitter<'a, S, E>
where
    S: Signer,
    E: Encrypter,
{
    /// Maximum size allowed for a packet
    transmission_size: usize,

    /// Disassembler used to break up data into packets
    disassembler: RwLock<Disassembler>,

    /// Performs authentication on data
    signer: &'a S,

    /// Performs encryption/decryption on data
    encrypter: &'a E,
}

impl<'a, S, E> Transmitter<'a, S, E>
where
    S: Signer,
    E: Encrypter,
{
    /// Begins building a transmitter, enabling us to specify options
    pub fn new(transmission_size: usize, signer: &'a S, encrypter: &'a E) -> Self {
        Self {
            transmission_size,
            signer,
            encrypter,
            disassembler: RwLock::new(Disassembler::new()),
        }
    }

    pub fn send(
        &self,
        data: &[u8],
        mut send_handler: impl FnMut(&[u8]) -> Result<(), IoError>,
    ) -> Result<(), TransmitterError> {
        // Encrypt entire dataset before splitting as it will grow in size
        // and it's difficult to predict if we can stay under our transmission
        // limit if encrypting at the individual packet level
        let associated_data = self.encrypter.new_encrypt_associated_data();
        let nonce = associated_data.nonce().cloned();
        let data = self
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
        let packets = {
            let mut d = self.disassembler.write().unwrap();
            d.make_packets_from_data(DisassembleInfo {
                id,
                nonce,
                data: &data,
                desired_chunk_size: self.transmission_size,
                signer: self.signer,
            })
            .map_err(TransmitterError::DisassembleData)?
        };

        // For each packet, serialize and send to specific address
        for packet in packets.iter() {
            let packet_data = packet.to_vec().map_err(TransmitterError::EncodePacket)?;
            send_handler(&packet_data).map_err(TransmitterError::SendBytes)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use over_there_auth::NoopAuthenticator;
    use over_there_crypto::NoopBicrypter;
    use std::io::ErrorKind as IoErrorKind;

    /// Uses no encryption or signing
    fn transmitter_with_transmission_size<'a>(
        transmission_size: usize,
    ) -> Transmitter<'a, NoopAuthenticator, NoopBicrypter> {
        Transmitter::new(transmission_size, &NoopAuthenticator, &NoopBicrypter)
    }

    #[test]
    fn send_should_fail_if_unable_to_convert_bytes_to_packets() {
        // Produce a transmitter with a "bytes per packet" that is too
        // low, causing the process to fail
        let m = transmitter_with_transmission_size(0);
        let data = vec![1, 2, 3];

        match m.send(&data, |_| Ok(())) {
            Err(TransmitterError::DisassembleData(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn send_should_fail_if_fails_to_send_bytes() {
        let m = transmitter_with_transmission_size(100);
        let data = vec![1, 2, 3];

        match m.send(&data, |_| Err(IoError::from(IoErrorKind::Other))) {
            Err(TransmitterError::SendBytes(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn send_should_return_okay_if_successfully_sent_data() {
        let m = transmitter_with_transmission_size(100);
        let data = vec![1, 2, 3];

        let result = m.send(&data, |_| Ok(()));
        assert_eq!(result.is_ok(), true);
    }

    #[cfg(test)]
    mod crypt {
        use super::*;
        use over_there_crypto::{AssociatedData, CryptError, Encrypter};

        struct BadEncrypter;
        impl Encrypter for BadEncrypter {
            fn encrypt(&self, _: &[u8], _: &AssociatedData) -> Result<Vec<u8>, CryptError> {
                Err(CryptError::EncryptFailed(From::from("Some error")))
            }

            fn new_encrypt_associated_data(&self) -> AssociatedData {
                AssociatedData::None
            }
        }

        #[test]
        fn send_should_fail_if_unable_to_encrypt_data() {
            let m = Transmitter::new(100, &NoopAuthenticator, &BadEncrypter);
            let data = vec![1, 2, 3];

            match m.send(&data, |_| Ok(())) {
                Err(super::TransmitterError::EncryptData(_)) => (),
                x => panic!("Unexpected result: {:?}", x),
            }
        }
    }
}
