use crate::{
    assembler::{self, Assembler},
    packet::Packet,
    transceiver::TransceiverContext,
};
use over_there_auth::{Signer, Verifier};
use over_there_crypto::{AssociatedData, CryptError, Decrypter, Encrypter, Nonce};
use over_there_derive::Error;
use std::io;

#[derive(Debug, Error)]
pub enum ReceiverError {
    DecodePacket(rmp_serde::decode::Error),
    UnableToVerifySignature,
    InvalidPacketSignature,
    AssembleData(assembler::AssemblerError),
    DecryptData(CryptError),
    RecvBytes(io::Error),
}

pub(crate) struct ReceiverContext<'a, V, D>
where
    V: Verifier,
    D: Decrypter,
{
    /// Buffer to contain bytes for temporary storage
    buffer: &'a mut [u8],

    /// Assembler used to gather packets together
    assembler: &'a mut Assembler,

    /// Performs verification on data
    verifier: &'a V,

    /// Performs encryption/decryption on data
    decrypter: &'a D,
}

impl<'a, A, B> From<&'a mut TransceiverContext<A, B>> for ReceiverContext<'a, A, B>
where
    A: Signer + Verifier,
    B: Encrypter + Decrypter,
{
    fn from(ctx: &'a mut TransceiverContext<A, B>) -> Self {
        Self {
            buffer: &mut ctx.buffer,
            assembler: &mut ctx.assembler,
            verifier: &ctx.authenticator,
            decrypter: &ctx.bicrypter,
        }
    }
}

pub(crate) fn do_receive<V, D, T, R>(
    ctx: ReceiverContext<'_, V, D>,
    read: R,
) -> Result<Option<(Vec<u8>, T)>, ReceiverError>
where
    V: Verifier,
    D: Decrypter,
    R: FnOnce(&mut [u8]) -> Result<(usize, T), io::Error>,
{
    // When retrieving bytes, check if we received an error indicating this
    // is async and that we should consider it as nothing
    let (size, other_data) = match read(ctx.buffer) {
        // Cases where we get zero bytes or a blocking error, we skip
        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(None),
        Ok((size, _)) if size == 0 => return Ok(None),

        // Otherwise, if we get a real error, we bubble it up
        Err(e) => return Err(ReceiverError::RecvBytes(e)),

        // Finally, if we get data, we continue
        Ok(x) => x,
    };
    println!("do_receive: {} bytes", size);

    // Process the received packet
    let p = Packet::from_slice(&ctx.buffer[..size]).map_err(ReceiverError::DecodePacket)?;
    println!("do_receive: data {:?}", p.data());

    // Verify the packet's signature, skipping any form of assembly if
    // it is not a legit packet
    if !verify_packet(ctx.verifier, &p)? {
        return Err(ReceiverError::InvalidPacketSignature);
    }

    let group_id = p.id();
    let nonce = p.nonce().cloned();
    println!("do_receive: group {} | {:?}", group_id, nonce);

    // Ensure that packet groups are still valid
    ctx.assembler.remove_expired();

    // Add the packet, see if we are ready to assemble the data, and do so
    let do_assemble = add_packet_and_verify(ctx.assembler, p)?;
    if do_assemble {
        println!("do_receive: assemble {}", group_id);

        // Gather the complete data
        let data = assemble_and_decrypt(group_id, ctx.assembler, ctx.decrypter, nonce)?;

        // Remove the underlying group as we no longer need to keep it
        ctx.assembler.remove_group(group_id);

        Ok(Some((data, other_data)))
    } else {
        Ok(None)
    }
}

fn verify_packet<V>(verifier: &V, packet: &Packet) -> Result<bool, ReceiverError>
where
    V: Verifier,
{
    let signature = packet.signature();
    let content = packet
        .content_for_signature()
        .map_err(|_| ReceiverError::UnableToVerifySignature)?;
    Ok(verifier.verify(&content, signature))
}

/// Adds the packet to our internal cache and checks to see if we
/// are ready to assemble the packet
fn add_packet_and_verify(assembler: &mut Assembler, packet: Packet) -> Result<bool, ReceiverError> {
    let id = packet.id();

    // Bubble up the error; we don't care about the success
    assembler
        .add_packet(packet)
        .map_err(ReceiverError::AssembleData)?;

    Ok(assembler.verify(id))
}

/// Assembles the complete data held by the assembler and decrypts it
/// using the internal bicrypter
fn assemble_and_decrypt<D>(
    group_id: u32,
    assembler: &Assembler,
    decrypter: &D,
    nonce: Option<Nonce>,
) -> Result<Vec<u8>, ReceiverError>
where
    D: Decrypter,
{
    // Assemble our data, which could be encrypted
    let data = assembler
        .assemble(group_id)
        .map_err(ReceiverError::AssembleData)?;

    // Decrypt our collective data
    let data = decrypter
        .decrypt(&data, &AssociatedData::from(nonce))
        .map_err(ReceiverError::DecryptData)?;

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::disassembler::{DisassembleInfo, Disassembler};
    use crate::packet::{PacketEncryption, PacketType};
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
    fn do_receive_should_fail_if_socket_fails_to_get_bytes() {
        let mut ctx = new_context(100);
        let rctx = From::from(&mut ctx);

        // NOTE: Having to produce this ugly failure function to get
        //       write function parameter to be created
        let f = |_: &mut [u8]| {
            if false {
                Ok((0, ()))
            } else {
                Err(io::Error::from(IoErrorKind::Other))
            }
        };

        match do_receive(rctx, f) {
            Err(ReceiverError::RecvBytes(_)) => (),
            Err(x) => panic!("Unexpected error: {:?}", x),
            Ok(x) => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn do_receive_should_fail_if_unable_to_convert_bytes_to_packet() {
        let mut ctx = new_context(100);
        let rctx = From::from(&mut ctx);

        // Force buffer to have a couple of early zeros, which is not
        // valid data when decoding
        match do_receive(rctx, |buf| {
            buf[0] = 0;
            buf[1] = 0;
            buf[2] = 0;
            Ok((buf.len(), ()))
        }) {
            Err(ReceiverError::DecodePacket(_)) => (),
            Err(x) => panic!("Unexpected error: {:?}", x),
            Ok(x) => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn do_receive_should_fail_if_unable_to_add_packet_to_assembler() {
        let mut ctx = new_context(100);
        let id = 0;
        let encryption = PacketEncryption::None;
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let signer = NoopAuthenticator;

        // Calculate the bigger of the two overhead sizes (final packet)
        // and ensure that we can fit data in it
        let overhead_size = Disassembler::estimate_packet_overhead_size(
            /* data size */ 1,
            PacketType::Final { encryption },
            &signer,
        )
        .unwrap();

        // Make several packets so that we don't send a single and last
        // packet, which would remove itself from the cache and allow
        // us to re-add a packet with the same id & index
        let p = &Disassembler::default()
            .make_packets_from_data(DisassembleInfo {
                id,
                encryption,
                data: &data,
                desired_chunk_size: overhead_size + 1,
                signer: &signer,
            })
            .unwrap()[0];
        let data = p.to_vec().unwrap();
        assert_eq!(
            do_receive(From::from(&mut ctx), |buf| {
                let l = data.len();
                buf[..l].clone_from_slice(&data);
                Ok((l, ()))
            })
            .is_ok(),
            true,
            "Failed to receive first packet!"
        );

        // Add the same packet more than once, which should
        // trigger the assembler to fail
        match do_receive(From::from(&mut ctx), |buf| {
            let l = data.len();
            buf[..l].clone_from_slice(&data);
            Ok((l, ()))
        }) {
            Err(ReceiverError::AssembleData(_)) => (),
            Err(x) => panic!("Unexpected error: {:?}", x),
            Ok(x) => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn do_receive_should_return_none_if_zero_bytes_received() {
        let mut ctx = new_context(100);
        let rctx = From::from(&mut ctx);

        match do_receive(rctx, |_| Ok((0, ()))) {
            Ok(None) => (),
            Ok(Some(x)) => panic!("Unexpected result: {:?}", x),
            Err(x) => panic!("Unexpected error: {:?}", x),
        }
    }

    #[test]
    fn do_receive_should_return_none_if_received_packet_does_not_complete_data() {
        let mut ctx = new_context(100);
        let rctx = From::from(&mut ctx);

        let id = 0;
        let encryption = PacketEncryption::None;
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let signer = NoopAuthenticator;

        // Calculate the bigger of the two overhead sizes (final packet)
        // and ensure that we can fit data in it
        let overhead_size = Disassembler::estimate_packet_overhead_size(
            /* data size */ 1,
            PacketType::Final { encryption },
            &signer,
        )
        .unwrap();

        // Make several packets so that we don't send a single and last
        // packet, which would result in a complete message
        let p = &Disassembler::default()
            .make_packets_from_data(DisassembleInfo {
                id,
                encryption,
                data: &data,
                desired_chunk_size: overhead_size + 1,
                signer: &NoopAuthenticator,
            })
            .unwrap()[0];
        let data = p.to_vec().unwrap();
        match do_receive(rctx, |buf| {
            let l = data.len();
            buf[..l].clone_from_slice(&data);
            Ok((l, ()))
        }) {
            Ok(None) => (),
            Ok(Some(x)) => panic!("Unexpected result: {:?}", x),
            Err(x) => panic!("Unexpected error: {:?}", x),
        }
    }

    #[test]
    fn do_receive_should_return_some_data_if_received_packet_does_complete_data() {
        let mut ctx = new_context(100);
        let rctx = From::from(&mut ctx);
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

        // Make one large packet so we can complete a message
        let p = &Disassembler::default()
            .make_packets_from_data(DisassembleInfo {
                id: 0,
                encryption: PacketEncryption::None,
                data: &data,
                desired_chunk_size: 100,
                signer: &NoopAuthenticator,
            })
            .unwrap()[0];
        let pdata = p.to_vec().unwrap();
        match do_receive(rctx, |buf| {
            let l = pdata.len();
            buf[..l].clone_from_slice(&pdata);
            Ok((l, ()))
        }) {
            Ok(Some((do_receive_data, _))) => {
                assert_eq!(
                    do_receive_data, data,
                    "Received unexpected data: {:?}",
                    do_receive_data
                );
            }
            Ok(None) => panic!("Unexpectedly received no data"),
            Err(x) => panic!("Unexpected error: {:?}", x),
        }
    }

    #[test]
    fn do_receive_should_remove_expired_packet_groups() {
        // Create a custom context whose packet groups within its assembler
        // will expire immediately
        let mut ctx =
            TransceiverContext::new(100, Duration::new(0, 0), NoopAuthenticator, NoopBicrypter);
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

        // Make many small packets
        let packets = &mut Disassembler::default()
            .make_packets_from_data(DisassembleInfo {
                id: 0,
                encryption: PacketEncryption::None,
                data: &data,
                desired_chunk_size: Disassembler::estimate_packet_overhead_size(
                    data.len(),
                    PacketType::NotFinal,
                    &NoopAuthenticator,
                )
                .unwrap()
                    + data.len(),
                signer: &NoopAuthenticator,
            })
            .unwrap();
        assert!(packets.len() > 1, "Did not produce many small packets");

        while !packets.is_empty() {
            let rctx = From::from(&mut ctx);
            assert!(
                do_receive(rctx, |buf| {
                    let pdata = packets.remove(0).to_vec().unwrap();
                    let l = pdata.len();
                    buf[..l].clone_from_slice(&pdata);
                    Ok((l, ()))
                })
                .unwrap()
                .is_none(),
                "Unexpectedly got result from receive with ttl of zero"
            );
        }
    }

    #[test]
    fn do_receive_should_remove_the_assembler_packet_group_if_does_complete_data() {
        let mut ctx = new_context(100);
        let rctx = From::from(&mut ctx);
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

        // Make one large packet so we can complete a message
        let p = &Disassembler::default()
            .make_packets_from_data(DisassembleInfo {
                id: 0,
                encryption: PacketEncryption::None,
                data: &data,
                desired_chunk_size: 100,
                signer: &NoopAuthenticator,
            })
            .unwrap()[0];
        let pdata = p.to_vec().unwrap();
        do_receive(rctx, |buf| {
            let l = pdata.len();
            buf[..l].clone_from_slice(&pdata);
            Ok((l, ()))
        })
        .unwrap();

        assert_eq!(ctx.assembler.len(), 0);
    }

    #[cfg(test)]
    mod crypt {
        use super::*;
        use over_there_crypto::{CryptError, Decrypter, Encrypter};

        struct BadDecrypter;
        impl Decrypter for BadDecrypter {
            fn decrypt(&self, _: &[u8], _: &AssociatedData) -> Result<Vec<u8>, CryptError> {
                Err(CryptError::DecryptFailed(From::from("Some error")))
            }
        }
        impl Encrypter for BadDecrypter {
            fn encrypt(&self, _: &[u8], _: &AssociatedData) -> Result<Vec<u8>, CryptError> {
                Err(CryptError::EncryptFailed(From::from("Some error")))
            }

            fn new_encrypt_associated_data(&self) -> over_there_crypto::AssociatedData {
                over_there_crypto::AssociatedData::None
            }
        }

        fn new_context(buffer_size: usize) -> TransceiverContext<NoopAuthenticator, BadDecrypter> {
            TransceiverContext::new(
                buffer_size,
                Duration::from_secs(1),
                NoopAuthenticator,
                BadDecrypter,
            )
        }

        #[test]
        fn do_receive_should_fail_if_unable_to_decrypt_data() {
            let mut ctx = new_context(100);

            let id = 0;
            let encryption = PacketEncryption::None;
            let data = vec![1, 2, 3];
            let signer = NoopAuthenticator;

            // Calculate the bigger of the two overhead sizes (final packet)
            // and ensure that we can fit data in it
            let overhead_size = Disassembler::estimate_packet_overhead_size(
                /* data size */ 1,
                PacketType::Final { encryption },
                &signer,
            )
            .unwrap();

            // Make a new packet per element in data
            let packets = Disassembler::default()
                .make_packets_from_data(DisassembleInfo {
                    id,
                    encryption,
                    data: &data.clone(),
                    desired_chunk_size: overhead_size + 1,
                    signer: &NoopAuthenticator,
                })
                .unwrap();

            // First N-1 packets should succeed
            for p in packets[..packets.len() - 1].iter() {
                let pdata = p.to_vec().unwrap();
                assert_eq!(
                    do_receive(From::from(&mut ctx), |buf| {
                        let l = pdata.len();
                        buf[..l].clone_from_slice(&pdata);
                        Ok((l, ()))
                    })
                    .is_ok(),
                    true,
                    "Unexpectedly failed to receive packet"
                );
            }

            // Final packet should trigger decrypting and it should fail
            let final_packet = packets.last().unwrap();
            let pdata = final_packet.to_vec().unwrap();
            match do_receive(From::from(&mut ctx), |buf| {
                let l = pdata.len();
                buf[..l].clone_from_slice(&pdata);
                Ok((l, ()))
            }) {
                Err(super::ReceiverError::DecryptData(_)) => (),
                Err(x) => panic!("Unexpected error: {:?}", x),
                Ok(x) => panic!("Unexpected result: {:?}", x),
            }
        }
    }
}
