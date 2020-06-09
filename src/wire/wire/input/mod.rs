pub mod decoder;

use crate::wire::wire::packet::Packet;
use decoder::Decoder;
use over_there_auth::Verifier;
use over_there_crypto::{AssociatedData, CryptError, Decrypter, Nonce};
use over_there_derive::Error;
use std::time::Duration;

#[derive(Debug, Error)]
pub enum InputProcessorError {
    EncodePacket(serde_cbor::Error),
    UnableToVerifySignature,
    InvalidPacketSignature,
    DecodeData(decoder::DecoderError),
    DecryptData(CryptError),
}

#[derive(Debug, Clone)]
pub struct InputProcessor<V, D>
where
    V: Verifier,
    D: Decrypter,
{
    decoder: Decoder,
    verifier: V,
    decrypter: D,
}

impl<V, D> InputProcessor<V, D>
where
    V: Verifier,
    D: Decrypter,
{
    pub fn new(packet_ttl: Duration, verifier: V, decrypter: D) -> Self {
        let decoder = Decoder::new(packet_ttl);
        Self {
            decoder,
            verifier,
            decrypter,
        }
    }

    pub fn process(
        &mut self,
        data: &[u8],
    ) -> Result<Option<Vec<u8>>, InputProcessorError> {
        if data.is_empty() {
            return Ok(None);
        }

        // Process the data as a packet
        let p = Packet::from_slice(data)
            .map_err(InputProcessorError::EncodePacket)?;

        // Verify the packet's signature, skipping any form of assembly if
        // it is not a legit packet
        if !verify_packet(&self.verifier, &p)? {
            return Err(InputProcessorError::InvalidPacketSignature);
        }

        let group_id = p.id();
        let nonce = p.nonce().cloned();

        // Ensure that packet groups are still valid
        self.decoder.remove_expired();

        // Add the packet, see if we are ready to decode the data, and do so
        let do_decode = add_packet_and_verify(&mut self.decoder, p)?;
        if do_decode {
            // Gather the complete data
            let data = decode_and_decrypt(
                group_id,
                &self.decoder,
                &self.decrypter,
                nonce,
            )?;

            // Remove the underlying group as we no longer need to keep it
            self.decoder.remove_group(group_id);

            Ok(Some(data))
        } else {
            Ok(None)
        }
    }
}

fn verify_packet<V>(
    verifier: &V,
    packet: &Packet,
) -> Result<bool, InputProcessorError>
where
    V: Verifier,
{
    let signature = packet.signature();
    let content = packet
        .content_for_signature()
        .map_err(|_| InputProcessorError::UnableToVerifySignature)?;
    Ok(verifier.verify(&content, signature))
}

/// Adds the packet to our internal cache and checks to see if we
/// are ready to decode the packet
fn add_packet_and_verify(
    decoder: &mut Decoder,
    packet: Packet,
) -> Result<bool, InputProcessorError> {
    let id = packet.id();

    // Bubble up the error; we don't care about the success
    decoder
        .add_packet(packet)
        .map_err(InputProcessorError::DecodeData)?;

    Ok(decoder.verify(id))
}

/// Decodes the complete data held by the decoder and decrypts it
/// using the internal bicrypter
fn decode_and_decrypt<D>(
    group_id: u32,
    decoder: &Decoder,
    decrypter: &D,
    nonce: Option<Nonce>,
) -> Result<Vec<u8>, InputProcessorError>
where
    D: Decrypter,
{
    // Decode our data, which could be encrypted
    let data = decoder
        .decode(group_id)
        .map_err(InputProcessorError::DecodeData)?;

    // Decrypt our collective data
    let data = decrypter
        .decrypt(&data, &AssociatedData::from(nonce))
        .map_err(InputProcessorError::DecryptData)?;

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::wire::{
        output::encoder::{EncodeArgs, Encoder},
        packet::{PacketEncryption, PacketType},
    };
    use over_there_auth::NoopAuthenticator;
    use over_there_crypto::NoopBicrypter;
    use std::time::Duration;

    fn new_processor() -> InputProcessor<NoopAuthenticator, NoopBicrypter> {
        InputProcessor::new(
            Duration::from_secs(1),
            NoopAuthenticator,
            NoopBicrypter,
        )
    }

    #[test]
    fn input_processor_process_should_fail_if_unable_to_convert_bytes_to_packet(
    ) {
        let mut processor = new_processor();

        match processor.process(&[0; 5]) {
            Err(InputProcessorError::EncodePacket(_)) => (),
            Err(x) => panic!("Unexpected error: {:?}", x),
            Ok(x) => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn input_processor_process_should_fail_if_unable_to_add_packet_to_decoder()
    {
        let mut processor = new_processor();
        let id = 0;
        let encryption = PacketEncryption::None;
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let signer = NoopAuthenticator;
        let mut encoder = Encoder::default();

        // Calculate a packet size where the final packet can only
        // fit a single byte of data to ensure that we get at least
        // one additional packet
        let max_packet_size = encoder
            .estimate_packet_size(
                /* data size */ 1,
                PacketType::Final { encryption },
                &signer,
            )
            .unwrap();

        // Make several packets so that we don't send a single and last
        // packet, which would remove itself from the cache and allow
        // us to re-add a packet with the same id & index
        let p = &encoder
            .encode(EncodeArgs {
                id,
                encryption,
                data: &data,
                max_packet_size,
                signer: &signer,
            })
            .unwrap()[0];
        let data = p.to_vec().unwrap();
        assert_eq!(
            processor.process(&data).is_ok(),
            true,
            "Failed to receive first packet!"
        );

        // Add the same packet more than once, which should
        // trigger the decoder to fail
        match processor.process(&data) {
            Err(InputProcessorError::DecodeData(_)) => (),
            Err(x) => panic!("Unexpected error: {:?}", x),
            Ok(x) => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn input_processor_process_should_return_none_if_zero_bytes_received() {
        let mut processor = new_processor();

        match processor.process(&[0; 0]) {
            Ok(None) => (),
            Ok(Some(x)) => panic!("Unexpected result: {:?}", x),
            Err(x) => panic!("Unexpected error: {:?}", x),
        }
    }

    #[test]
    fn input_processor_process_should_return_none_if_received_packet_does_not_complete_data(
    ) {
        let mut processor = new_processor();

        let id = 0;
        let encryption = PacketEncryption::None;
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let signer = NoopAuthenticator;
        let mut encoder = Encoder::default();

        // Calculate a packet size where the final packet can only
        // fit a single byte of data to ensure that we get at least
        // one additional packet
        let max_packet_size = encoder
            .estimate_packet_size(
                /* data size */ 1,
                PacketType::Final { encryption },
                &signer,
            )
            .unwrap();

        // Make several packets so that we don't send a single and last
        // packet, which would result in a complete message
        let p = &encoder
            .encode(EncodeArgs {
                id,
                encryption,
                data: &data,
                max_packet_size,
                signer: &NoopAuthenticator,
            })
            .unwrap()[0];
        let data = p.to_vec().unwrap();
        match processor.process(&data) {
            Ok(None) => (),
            Ok(Some(x)) => panic!("Unexpected result: {:?}", x),
            Err(x) => panic!("Unexpected error: {:?}", x),
        }
    }

    #[test]
    fn input_processor_process_should_return_some_data_if_received_packet_does_complete_data(
    ) {
        let mut processor = new_processor();
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

        // Make one large packet so we can complete a message
        let p = &Encoder::default()
            .encode(EncodeArgs {
                id: 0,
                encryption: PacketEncryption::None,
                data: &data,
                max_packet_size: 100,
                signer: &NoopAuthenticator,
            })
            .unwrap()[0];
        let pdata = p.to_vec().unwrap();
        match processor.process(&pdata) {
            Ok(Some(input_processor_process_data)) => {
                assert_eq!(
                    input_processor_process_data, data,
                    "Received unexpected data: {:?}",
                    input_processor_process_data
                );
            }
            Ok(None) => panic!("Unexpectedly received no data"),
            Err(x) => panic!("Unexpected error: {:?}", x),
        }
    }

    #[test]
    fn input_processor_process_should_remove_expired_packet_groups() {
        // Create a custom context whose packet groups within its decoder
        // will expire immediately
        let mut processor = InputProcessor::new(
            Duration::new(0, 0),
            NoopAuthenticator,
            NoopBicrypter,
        );
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let mut encoder = Encoder::default();

        // Make many small packets
        let packets = &mut Encoder::default()
            .encode(EncodeArgs {
                id: 0,
                encryption: PacketEncryption::None,
                data: &data,
                max_packet_size: encoder
                    .estimate_packet_size(
                        /* data size for final packet */ 1,
                        PacketType::NotFinal,
                        &NoopAuthenticator,
                    )
                    .unwrap()
                    + data.len(),
                signer: &NoopAuthenticator,
            })
            .unwrap();
        assert!(packets.len() > 1, "Did not produce many small packets");

        for p in packets.iter() {
            let pdata = p.to_vec().unwrap();
            assert!(
                processor.process(&pdata).unwrap().is_none(),
                "Unexpectedly got result from process with ttl of zero"
            );
        }
    }

    #[test]
    fn input_processor_process_should_remove_the_decoder_packet_group_if_does_complete_data(
    ) {
        let mut processor = new_processor();
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

        // Make one large packet so we can complete a message
        let p = &Encoder::default()
            .encode(EncodeArgs {
                id: 0,
                encryption: PacketEncryption::None,
                data: &data,
                max_packet_size: 100,
                signer: &NoopAuthenticator,
            })
            .unwrap()[0];
        let pdata = p.to_vec().unwrap();
        processor.process(&pdata).unwrap();

        assert_eq!(processor.decoder.len(), 0);
    }

    #[cfg(test)]
    mod crypt {
        use super::*;
        use over_there_crypto::{CryptError, Decrypter, Encrypter};

        #[derive(Clone)]
        struct BadDecrypter;
        impl Decrypter for BadDecrypter {
            fn decrypt(
                &self,
                _: &[u8],
                _: &AssociatedData,
            ) -> Result<Vec<u8>, CryptError> {
                Err(CryptError::DecryptFailed(From::from("Some error")))
            }
        }
        impl Encrypter for BadDecrypter {
            fn encrypt(
                &self,
                _: &[u8],
                _: &AssociatedData,
            ) -> Result<Vec<u8>, CryptError> {
                Err(CryptError::EncryptFailed(From::from("Some error")))
            }

            fn new_encrypt_associated_data(
                &self,
            ) -> over_there_crypto::AssociatedData {
                over_there_crypto::AssociatedData::None
            }
        }

        fn new_processor() -> InputProcessor<NoopAuthenticator, BadDecrypter> {
            InputProcessor::new(
                Duration::from_secs(1),
                NoopAuthenticator,
                BadDecrypter,
            )
        }

        #[test]
        fn input_processor_process_should_fail_if_unable_to_decrypt_data() {
            let mut processor = new_processor();

            let id = 0;
            let encryption = PacketEncryption::None;
            let data = vec![1, 2, 3];
            let signer = NoopAuthenticator;
            let mut encoder = Encoder::default();

            // Calculate a packet size where the final packet can only
            // fit a single byte of data to ensure that we get at least
            // one additional packet
            let max_packet_size = encoder
                .estimate_packet_size(
                    /* data size */ 1,
                    PacketType::Final { encryption },
                    &signer,
                )
                .unwrap();

            // Make a new packet per element in data
            let packets = encoder
                .encode(EncodeArgs {
                    id,
                    encryption,
                    data: &data.clone(),
                    max_packet_size,
                    signer: &NoopAuthenticator,
                })
                .unwrap();

            // First N-1 packets should succeed
            for p in packets[..packets.len() - 1].iter() {
                let pdata = p.to_vec().unwrap();
                assert_eq!(
                    processor.process(&pdata).is_ok(),
                    true,
                    "Unexpectedly failed to receive packet"
                );
            }

            // Final packet should trigger decrypting and it should fail
            let final_packet = packets.last().unwrap();
            let pdata = final_packet.to_vec().unwrap();
            match processor.process(&pdata) {
                Err(super::InputProcessorError::DecryptData(_)) => (),
                Err(x) => panic!("Unexpected error: {:?}", x),
                Ok(x) => panic!("Unexpected result: {:?}", x),
            }
        }
    }
}
