use crate::{
    assembler::{self, Assembler},
    disassembler::{self, Disassembler},
    packet::Packet,
};
use over_there_crypto::{AssociatedData, Bicrypter, CryptError, Nonce};
use over_there_derive::Error;
use rand::random;
use std::cell::RefCell;
use std::io::Error as IoError;
use std::time::Duration;
use ttl_cache::TtlCache;

#[derive(Debug, Error)]
pub enum TransmitterError {
    EncodePacket(rmp_serde::encode::Error),
    DecodePacket(rmp_serde::decode::Error),
    AssembleData(assembler::AssemblerError),
    DisassembleData(disassembler::DisassemblerError),
    EncryptData(CryptError),
    DecryptData(CryptError),
    SendBytes(IoError),
    RecvBytes(IoError),
}

pub struct Transmitter<B>
where
    B: Bicrypter,
{
    /// Maximum size allowed for a packet
    transmission_size: usize,

    /// Cache of packets belonging to a group that has not been completed
    cache: RefCell<TtlCache<u32, Assembler>>,

    /// Maximum time for a cache entry to exist untouched before expiring
    cache_duration: Duration,

    /// Buffer to contain bytes for temporary storage
    /// NOTE: Cannot use static array due to type constraints
    buffer: RefCell<Box<[u8]>>,

    /// Performs encryption/decryption on data
    bicrypter: B,
}

impl<B> Transmitter<B>
where
    B: Bicrypter,
{
    /// Begins building a transmitter, enabling us to specify options
    pub fn new(
        transmission_size: usize,
        cache_capacity: usize,
        cache_duration: Duration,
        bicrypter: B,
    ) -> Self {
        let cache = RefCell::new(TtlCache::new(cache_capacity));
        let buffer = RefCell::new(vec![0; transmission_size as usize].into_boxed_slice());
        Transmitter {
            transmission_size,
            cache,
            buffer,
            bicrypter,
            cache_duration,
        }
    }

    pub fn send(
        &self,
        data: Vec<u8>,
        mut send_handler: impl FnMut(Vec<u8>) -> Result<(), IoError>,
    ) -> Result<(), TransmitterError> {
        // Encrypt entire dataset before splitting as it will grow in size
        // and it's difficult to predict if we can stay under our transmission
        // limit if encrypting at the individual packet level
        let associated_data = self.bicrypter.new_encrypt_associated_data();
        let nonce = associated_data.nonce().cloned();
        let data = self
            .bicrypter
            .encrypt(&data, &associated_data)
            .map_err(TransmitterError::EncryptData)?;

        // Produce a unique id used to group our packets
        let id: u32 = random();

        // Split data into multiple packets
        let packets = Disassembler::make_packets_from_data(id, nonce, data, self.transmission_size)
            .map_err(TransmitterError::DisassembleData)?;

        // For each packet, serialize and send to specific address
        for packet in packets.iter() {
            let packet_data = packet.to_vec().map_err(TransmitterError::EncodePacket)?;
            send_handler(packet_data).map_err(TransmitterError::SendBytes)?;
        }

        Ok(())
    }

    pub fn recv(
        &self,
        mut recv_handler: impl FnMut(&mut [u8]) -> Result<usize, IoError>,
    ) -> Result<Option<Vec<u8>>, TransmitterError> {
        let mut buf = self.buffer.borrow_mut();
        let size = recv_handler(&mut buf).map_err(TransmitterError::RecvBytes)?;

        // If we don't receive any bytes, we treat it as there are no bytes
        // available, which is not an error but also does not warrant trying
        // to parse a packet, which will cause an error
        if size == 0 {
            return Ok(None);
        }

        // Process the received packet
        let p = Packet::from_slice(&buf[..size]).map_err(TransmitterError::DecodePacket)?;
        let id = p.id();
        let nonce = p.nonce().cloned();

        // Retrieve or create assembler for packet group
        let mut cache = self.cache.borrow_mut();
        match Self::get_or_new_assembler(&mut cache, id, self.cache_duration) {
            Some(assembler) => {
                let do_assemble = Self::add_packet_and_verify(assembler, p)?;
                if do_assemble {
                    let data = Self::assemble_and_decrypt(assembler, &self.bicrypter, nonce)
                        .map(|d| Some(d));

                    // We also want to drop the assembler at this point
                    cache.remove(&id);

                    data
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    /// Retrieves an assembler using its id, or creates a new assembler;
    /// can yield None in the off chance that the assembler expires inbetween
    /// the time that it is created and returned
    fn get_or_new_assembler(
        cache: &mut TtlCache<u32, Assembler>,
        id: u32,
        cache_duration: Duration,
    ) -> Option<&mut Assembler> {
        // Trigger removal of expired items in cache
        // NOTE: This is a hack given that the call to remove_expired is private
        cache.iter();

        // TODO: Extend entry of ttl_cache to include .or_insert(), which
        // should fix issue returning mutable reference in another approaches
        if !cache.contains_key(&id) {
            cache.insert(id, Assembler::new(), cache_duration);
        }
        cache.get_mut(&id)
    }

    /// Adds the packet to our internal cache and checks to see if we
    /// are ready to assemble the packet
    fn add_packet_and_verify(
        assembler: &mut Assembler,
        packet: Packet,
    ) -> Result<bool, TransmitterError> {
        // Bubble up the error; we don't care about the success
        assembler
            .add_packet(packet)
            .map_err(TransmitterError::AssembleData)?;

        Ok(assembler.verify())
    }

    /// Assembles the complete data held by the assembler and decrypts it
    /// using the internal bicrypter
    fn assemble_and_decrypt(
        assembler: &Assembler,
        bicrypter: &B,
        nonce: Option<Nonce>,
    ) -> Result<Vec<u8>, TransmitterError> {
        // Assemble our data, which could be encrypted
        let data = assembler
            .assemble()
            .map_err(TransmitterError::AssembleData)?;

        // Decrypt our collective data
        let data = bicrypter
            .decrypt(&data, &AssociatedData::from(nonce))
            .map_err(TransmitterError::DecryptData)?;

        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use over_there_crypto::NoopBicrypter;
    use std::io::ErrorKind as IoErrorKind;

    const MAX_CACHE_CAPACITY: usize = 1500;
    const MAX_CACHE_DURATION_IN_SECS: u64 = 5 * 60;

    /// Uses no encryption or signing
    fn transmitter_with_transmission_size(transmission_size: usize) -> Transmitter<NoopBicrypter> {
        Transmitter::new(
            transmission_size,
            MAX_CACHE_CAPACITY,
            Duration::from_secs(MAX_CACHE_DURATION_IN_SECS),
            NoopBicrypter::new(),
        )
    }

    #[test]
    fn send_should_fail_if_unable_to_convert_bytes_to_packets() {
        // Produce a transmitter with a "bytes per packet" that is too
        // low, causing the process to fail
        let m = transmitter_with_transmission_size(0);
        let data = vec![1, 2, 3];

        match m.send(data, |_| Ok(())) {
            Err(TransmitterError::DisassembleData(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn send_should_fail_if_fails_to_send_bytes() {
        let m = transmitter_with_transmission_size(100);
        let data = vec![1, 2, 3];

        match m.send(data, |_| Err(IoError::from(IoErrorKind::Other))) {
            Err(TransmitterError::SendBytes(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn send_should_return_okay_if_successfully_sent_data() {
        let m = transmitter_with_transmission_size(100);
        let data = vec![1, 2, 3];

        let result = m.send(data, |_| Ok(()));
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    fn recv_should_fail_if_socket_fails_to_get_bytes() {
        let m = transmitter_with_transmission_size(100);

        match m.recv(|_| Err(IoError::from(IoErrorKind::Other))) {
            Err(TransmitterError::RecvBytes(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn recv_should_fail_if_unable_to_convert_bytes_to_packet() {
        let m = transmitter_with_transmission_size(100);

        // Force buffer to have a couple of early zeros, which is not
        // valid data when decoding
        match m.recv(|buf| {
            buf[0] = 0;
            buf[1] = 0;
            buf[2] = 0;
            Ok(buf.len())
        }) {
            Err(TransmitterError::DecodePacket(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn recv_should_fail_if_unable_to_add_packet_to_assembler() {
        let m = transmitter_with_transmission_size(100);

        // Make several packets so that we don't send a single and last
        // packet, which would remove itself from the cache and allow
        // us to re-add a packet with the same id & index
        let p = &Disassembler::make_packets_from_data(
            0,
            None,
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
            Packet::metadata_size() + 1,
        )
        .unwrap()[0];
        let data = p.to_vec().unwrap();
        assert_eq!(
            m.recv(|buf| {
                let l = data.len();
                buf[..l].clone_from_slice(&data);
                Ok(l)
            })
            .is_ok(),
            true,
            "Failed to receive first packet!"
        );

        // Add the same packet more than once, which should
        // trigger the assembler to fail
        match m.recv(|buf| {
            let l = data.len();
            buf[..l].clone_from_slice(&data);
            Ok(l)
        }) {
            Err(TransmitterError::AssembleData(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn recv_should_return_none_if_zero_bytes_received() {
        let m = transmitter_with_transmission_size(100);

        match m.recv(|_| Ok(0)) {
            Ok(None) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn recv_should_return_none_if_the_assembler_expired() {
        // Make a transmitter that has a really short duration
        let wait_duration = Duration::from_nanos(1);
        let m = Transmitter::new(100, 100, wait_duration, NoopBicrypter::new());

        // Make several packets so that we don't send a single and last
        // packet, which would result in a complete message
        let packets = &mut Disassembler::make_packets_from_data(
            0,
            None,
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
            Packet::metadata_size() + 1,
        )
        .unwrap();

        while !packets.is_empty() {
            match m.recv(|buf| {
                let p = packets.remove(0);
                let data = p.to_vec().unwrap();
                let l = data.len();
                buf[..l].clone_from_slice(&data);
                Ok(l)
            }) {
                Ok(Some(_)) if packets.is_empty() => {
                    panic!("Unexpectedly got complete message! Expiration did not happen")
                }
                Ok(Some(_)) => panic!(
                    "Unexpectedly got complete message with {} packets remaining",
                    packets.len()
                ),
                Ok(None) => (),
                x => panic!("Unexpected result: {:?}", x),
            }

            // Wait the same time as our expiration to make sure we throw
            // out the old packets
            std::thread::sleep(wait_duration);
        }
    }

    #[test]
    fn recv_should_return_none_if_received_packet_does_not_complete_data() {
        let m = transmitter_with_transmission_size(100);

        // Make several packets so that we don't send a single and last
        // packet, which would result in a complete message
        let p = &Disassembler::make_packets_from_data(
            0,
            None,
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
            Packet::metadata_size() + 1,
        )
        .unwrap()[0];
        let data = p.to_vec().unwrap();
        match m.recv(|buf| {
            let l = data.len();
            buf[..l].clone_from_slice(&data);
            Ok(l)
        }) {
            Ok(None) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn recv_should_return_some_data_if_received_packet_does_complete_data() {
        let m = transmitter_with_transmission_size(100);
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

        // Make one large packet so we can complete a message
        let p = &Disassembler::make_packets_from_data(0, None, data.clone(), 100).unwrap()[0];
        let pdata = p.to_vec().unwrap();
        match m.recv(|buf| {
            let l = pdata.len();
            buf[..l].clone_from_slice(&pdata);
            Ok(l)
        }) {
            Ok(Some(recv_data)) => {
                assert_eq!(recv_data, data, "Received unexpected data: {:?}", recv_data);
            }
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[cfg(test)]
    mod crypt {
        use super::*;
        use over_there_crypto::{CryptError, Decrypter, Encrypter};

        struct BadBicrypter;
        impl Bicrypter for BadBicrypter {}
        impl Encrypter for BadBicrypter {
            fn encrypt(&self, _: &[u8], _: &AssociatedData) -> Result<Vec<u8>, CryptError> {
                Err(CryptError::EncryptFailed(From::from("Some error")))
            }

            fn new_encrypt_associated_data(&self) -> AssociatedData {
                AssociatedData::None
            }
        }
        impl Decrypter for BadBicrypter {
            fn decrypt(&self, _: &[u8], _: &AssociatedData) -> Result<Vec<u8>, CryptError> {
                Err(CryptError::DecryptFailed(From::from("Some error")))
            }
        }

        #[test]
        fn recv_should_fail_if_unable_to_decrypt_data() {
            let m = Transmitter::new(
                100,
                MAX_CACHE_CAPACITY,
                Duration::from_secs(MAX_CACHE_DURATION_IN_SECS),
                BadBicrypter,
            );
            let data = vec![1, 2, 3];

            // Make a new packet per element in data
            let packets = Disassembler::make_packets_from_data(
                0,
                None,
                data.clone(),
                Packet::metadata_size() + 1,
            )
            .unwrap();

            // First N-1 packets should succeed
            for p in packets[..packets.len() - 1].iter() {
                let pdata = p.to_vec().unwrap();
                assert_eq!(
                    m.recv(|buf| {
                        let l = pdata.len();
                        buf[..l].clone_from_slice(&pdata);
                        Ok(l)
                    })
                    .is_ok(),
                    true,
                    "Unexpectedly failed to receive packet"
                );
            }

            // Final packet should trigger decrypting and it should fail
            let final_packet = packets.last().unwrap();
            let pdata = final_packet.to_vec().unwrap();
            match m.recv(|buf| {
                let l = pdata.len();
                buf[..l].clone_from_slice(&pdata);
                Ok(l)
            }) {
                Err(super::TransmitterError::DecryptData(_)) => (),
                x => panic!("Unexpected result: {:?}", x),
            }
        }

        #[test]
        fn send_should_fail_if_unable_to_encrypt_data() {
            let m = Transmitter::new(
                100,
                MAX_CACHE_CAPACITY,
                Duration::from_secs(MAX_CACHE_DURATION_IN_SECS),
                BadBicrypter,
            );
            let data = vec![1, 2, 3];

            match m.send(data, |_| Ok(())) {
                Err(super::TransmitterError::EncryptData(_)) => (),
                x => panic!("Unexpected result: {:?}", x),
            }
        }
    }
}
