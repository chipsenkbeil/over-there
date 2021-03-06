pub mod encoder;

use crate::core::transport::crypto::{CryptError, Encrypter};
use crate::core::transport::{auth::Signer, wire::packet::PacketEncryption};
use derive_more::{Display, Error};
use encoder::{EncodeArgs, Encoder};

#[derive(Debug, Display, Error)]
pub enum OutputProcessorError {
    DecodePacket(serde_cbor::Error),
    EncodeData(encoder::EncoderError),
    EncryptData(CryptError),
}

#[derive(Debug, Clone)]
pub struct OutputProcessor<S, E>
where
    S: Signer,
    E: Encrypter,
{
    encoder: Encoder,
    transmission_size: usize,
    signer: S,
    encrypter: E,
}

impl<S, E> OutputProcessor<S, E>
where
    S: Signer,
    E: Encrypter,
{
    pub fn new(transmission_size: usize, signer: S, encrypter: E) -> Self {
        let encoder = Encoder::default();
        Self {
            encoder,
            transmission_size,
            signer,
            encrypter,
        }
    }

    pub fn process(
        &mut self,
        data: &[u8],
    ) -> Result<Vec<Vec<u8>>, OutputProcessorError> {
        // Encrypt entire dataset before splitting as it will grow in size
        // and it's difficult to predict if we can stay under our transmission
        // limit if encrypting at the individual packet level
        let associated_data = self.encrypter.new_encrypt_associated_data();
        let encryption = PacketEncryption::from(associated_data.clone());
        let data = self
            .encrypter
            .encrypt(data, &associated_data)
            .map_err(OutputProcessorError::EncryptData)?;

        // Produce a unique id used to group our packets
        let id: u32 = Self::new_id();

        // Split data into multiple packets
        // NOTE: Must protect mutable access to encoder, which caches
        //       computing the estimated packet sizes; if there is a way
        //       that we could do this faster (not need a cache), we could
        //       get rid of the locking and only need a reference
        let packets = self
            .encoder
            .encode(EncodeArgs {
                id,
                encryption,
                data: &data,
                max_packet_size: self.transmission_size,
                signer: &self.signer,
            })
            .map_err(OutputProcessorError::EncodeData)?;

        // For each packet, serialize and add to output
        let mut output = Vec::new();
        for packet in packets.iter() {
            let packet_data = packet
                .to_vec()
                .map_err(OutputProcessorError::DecodePacket)?;
            output.push(packet_data);
        }

        Ok(output)
    }

    fn new_id() -> u32 {
        rand::random()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::transport::auth::{ClosureSigner, Digest, NoopAuthenticator};
    use crate::core::transport::crypto::{ClosureEncrypter, NoopBicrypter};
    use crate::core::transport::wire::packet::Packet;

    fn new_processor(
        buffer_size: usize,
    ) -> OutputProcessor<NoopAuthenticator, NoopBicrypter> {
        OutputProcessor::new(buffer_size, NoopAuthenticator, NoopBicrypter)
    }

    #[test]
    fn output_processor_process_should_fail_if_unable_to_convert_bytes_to_packets(
    ) {
        // Produce a transmitter with a "bytes per packet" that is too
        // low, causing the process to fail
        let mut processor = new_processor(0);
        let data = vec![1, 2, 3];

        match processor.process(&data) {
            Err(OutputProcessorError::EncodeData(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn output_processor_process_should_return_signed_and_encrypted_serialized_packets(
    ) {
        use std::convert::TryFrom;
        let signer = ClosureSigner::new(|_| Digest::try_from([9; 32]).unwrap());
        let encrypter = ClosureEncrypter::new(|msg, _| {
            let mut v = Vec::new();

            for d in msg {
                v.push(*d);
            }

            v.push(99);

            Ok(v)
        });
        let mut processor = OutputProcessor::new(100, signer, encrypter);
        let data = vec![1, 2, 3];

        match processor.process(&data) {
            Ok(packeted_data) => {
                assert_eq!(
                    packeted_data.len(),
                    1,
                    "More packets than expected"
                );
                let packet_bytes = &packeted_data[0];
                let packet = Packet::from_slice(packet_bytes).unwrap();

                assert_eq!(packet.signature().digest(), &[9; 32]);
                assert_eq!(packet.data(), &vec![1, 2, 3, 99]);
            }
            Err(x) => panic!("Unexpected error: {:?}", x),
        }
    }

    #[cfg(test)]
    mod crypt {
        use super::*;
        use crate::core::transport::crypto::{
            AssociatedData, CryptError, Decrypter, Encrypter,
        };

        #[derive(Clone)]
        struct BadEncrypter;
        impl Encrypter for BadEncrypter {
            fn encrypt(
                &self,
                _: &[u8],
                _: &AssociatedData,
            ) -> Result<Vec<u8>, CryptError> {
                Err(CryptError::EncryptFailed(From::from("Some error")))
            }

            fn new_encrypt_associated_data(&self) -> AssociatedData {
                AssociatedData::None
            }
        }
        impl Decrypter for BadEncrypter {
            fn decrypt(
                &self,
                _: &[u8],
                _: &AssociatedData,
            ) -> Result<Vec<u8>, CryptError> {
                Err(CryptError::DecryptFailed(From::from("Some error")))
            }
        }

        fn new_processor(
            buffer_size: usize,
        ) -> OutputProcessor<NoopAuthenticator, BadEncrypter> {
            OutputProcessor::new(buffer_size, NoopAuthenticator, BadEncrypter)
        }

        #[test]
        fn output_processor_process_should_fail_if_unable_to_encrypt_data() {
            let mut processor = new_processor(100);
            let data = vec![1, 2, 3];

            match processor.process(&data) {
                Err(super::OutputProcessorError::EncryptData(_)) => (),
                x => panic!("Unexpected result: {:?}", x),
            }
        }
    }
}
