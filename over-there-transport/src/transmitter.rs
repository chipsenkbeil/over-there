use crate::{
    assembler::{self, Assembler},
    disassembler::{self, Disassembler},
    packet::Packet,
};
use log::debug;
use over_there_crypto::{Bicrypter, Error as CryptError};
use rand::random;
use std::cell::RefCell;
use std::time::Duration;
use ttl_cache::TtlCache;

#[derive(Debug)]
pub enum Error {
    EncodePacket(rmp_serde::encode::Error),
    DecodePacket(rmp_serde::decode::Error),
    AssembleData(assembler::Error),
    DisassembleData(disassembler::Error),
    EncryptData(CryptError),
    DecryptData(CryptError),
    SendBytes(std::io::Error),
    RecvBytes(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &*self {
            Error::EncodePacket(error) => write!(f, "Failed to encode packet: {:?}", error),
            Error::DecodePacket(error) => write!(f, "Failed to decode packet: {:?}", error),
            Error::AssembleData(error) => write!(f, "Failed to assemble data: {:?}", error),
            Error::DisassembleData(error) => write!(f, "Failed to disassemble data: {:?}", error),
            Error::EncryptData(error) => write!(f, "Failed to encrypt data: {:?}", error),
            Error::DecryptData(error) => write!(f, "Failed to decrypt data: {:?}", error),
            Error::SendBytes(error) => write!(f, "Failed to send bytes: {:?}", error),
            Error::RecvBytes(error) => write!(f, "Failed to receive bytes: {:?}", error),
        }
    }
}

impl std::error::Error for Error {}

pub struct Transmitter {
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
    bicrypter: Box<dyn Bicrypter>,
}

impl Transmitter {
    /// Begins building a transmitter, enabling us to specify options
    pub fn new(
        transmission_size: usize,
        cache_capacity: usize,
        cache_duration: Duration,
        bicrypter: Box<dyn Bicrypter>,
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
        mut send_handler: impl FnMut(Vec<u8>) -> Result<(), std::io::Error>,
    ) -> Result<(), Error> {
        // Encrypt entire dataset before splitting as it will grow in size
        // and it's difficult to predict if we can stay under our transmission
        // limit if encrypting at the individual packet level
        let data = self.bicrypter.encrypt(&data).map_err(Error::EncryptData)?;

        // Produce a unique id used to group our packets
        let id: u32 = random();

        // Split data into multiple packets
        let packets = Disassembler::make_packets_from_data(id, data, self.transmission_size)
            .map_err(Error::DisassembleData)?;

        // For each packet, serialize and send to specific address
        for packet in packets.iter() {
            let packet_data = packet.to_vec().map_err(Error::EncodePacket)?;
            send_handler(packet_data).map_err(Error::SendBytes)?;
        }

        Ok(())
    }

    pub fn recv(
        &self,
        mut recv_handler: impl FnMut(&mut [u8]) -> Result<usize, std::io::Error>,
    ) -> Result<Option<Vec<u8>>, Error> {
        let mut buf = self.buffer.borrow_mut();
        let size = recv_handler(&mut buf).map_err(Error::RecvBytes)?;

        // If we don't receive any bytes, we treat it as there are no bytes
        // available, which is not an error but also does not warrant trying
        // to parse a packet, which will cause an error
        if size == 0 {
            return Ok(None);
        }
        debug!("{} incoming bytes", size);

        // Process the received packet
        let p = Packet::from_slice(&buf[..size]).map_err(Error::DecodePacket)?;
        let p_id = p.id();
        debug!(
            "Packet [id: {} | index: {} | is_last: {}]",
            p_id,
            p.index(),
            p.is_last()
        );

        // Grab a reference to our cache of packet assemblers that we will use; also drop any
        // expired assemblers
        let mut map = self.cache.borrow_mut();

        // Trigger removal of expired items in cache
        // NOTE: This is a hack given that the call to remove_expired is private
        map.iter();

        // Retrieve the assembler associated with the packet or
        // create a new instance
        let maybe_assembler = match map.get_mut(&p_id) {
            None => {
                map.insert(p.id(), Assembler::new(), self.cache_duration);
                map.get_mut(&p.id())
            }
            x => x,
        };

        match maybe_assembler {
            Some(assembler) => {
                // Bubble up the error; we don't care about the success
                assembler.add_packet(p).map_err(Error::AssembleData)?;

                // Determine if time to assemble message
                if assembler.verify() {
                    // Assemble and decrypt our collective data
                    let data = self
                        .bicrypter
                        .decrypt(&assembler.assemble().map_err(Error::AssembleData)?)
                        .map_err(Error::DecryptData)?;

                    // We also want to drop the assembler at this point
                    map.remove(&p_id);

                    Ok(Some(data))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use over_there_crypto::noop;

    const MAX_CACHE_CAPACITY: usize = 1500;
    const MAX_CACHE_DURATION_IN_SECS: u64 = 5 * 60;

    /// Uses no encryption or signing
    fn transmitter_with_transmission_size(transmission_size: usize) -> Transmitter {
        Transmitter::new(
            transmission_size,
            MAX_CACHE_CAPACITY,
            Duration::from_secs(MAX_CACHE_DURATION_IN_SECS),
            Box::new(noop::Bicrypter::new()),
        )
    }

    #[test]
    fn send_should_fail_if_unable_to_convert_bytes_to_packets() {
        // Produce a transmitter with a "bytes per packet" that is too
        // low, causing the process to fail
        let m = transmitter_with_transmission_size(0);
        let data = vec![1, 2, 3];

        match m.send(data, |_| Ok(())) {
            Err(Error::DisassembleData(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn send_should_fail_if_fails_to_send_bytes() {
        let m = transmitter_with_transmission_size(100);
        let data = vec![1, 2, 3];

        match m.send(data, |_| {
            Err(std::io::Error::from(std::io::ErrorKind::Other))
        }) {
            Err(Error::SendBytes(_)) => (),
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

        match m.recv(|_| Err(std::io::Error::from(std::io::ErrorKind::Other))) {
            Err(Error::RecvBytes(_)) => (),
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
            Err(Error::DecodePacket(_)) => (),
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
            Err(Error::AssembleData(_)) => (),
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
        let m = Transmitter::new(100, 100, wait_duration, Box::new(noop::Bicrypter::new()));

        // Make several packets so that we don't send a single and last
        // packet, which would result in a complete message
        let packets = &mut Disassembler::make_packets_from_data(
            0,
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
        let p = &Disassembler::make_packets_from_data(0, data.clone(), 100).unwrap()[0];
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
        use over_there_crypto::{Decrypter, Encrypter, Error};

        struct BadBicrypter {}
        impl Bicrypter for BadBicrypter {}
        impl Encrypter for BadBicrypter {
            fn encrypt(&self, _data: &[u8]) -> Result<Vec<u8>, Error> {
                Err(Error::Encrypt(From::from("Some error")))
            }
        }
        impl Decrypter for BadBicrypter {
            fn decrypt(&self, _data: &[u8]) -> Result<Vec<u8>, Error> {
                Err(Error::Decrypt(From::from("Some error")))
            }
        }

        #[test]
        fn recv_should_fail_if_unable_to_decrypt_data() {
            let m = Transmitter::new(
                100,
                MAX_CACHE_CAPACITY,
                Duration::from_secs(MAX_CACHE_DURATION_IN_SECS),
                Box::new(BadBicrypter {}),
            );
            let data = vec![1, 2, 3];

            // Make a new packet per element in data
            let packets =
                Disassembler::make_packets_from_data(0, data.clone(), Packet::metadata_size() + 1)
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
                Err(super::Error::DecryptData(_)) => (),
                x => panic!("Unexpected result: {:?}", x),
            }
        }

        #[test]
        fn send_should_fail_if_unable_to_encrypt_data() {
            let m = Transmitter::new(
                100,
                MAX_CACHE_CAPACITY,
                Duration::from_secs(MAX_CACHE_DURATION_IN_SECS),
                Box::new(BadBicrypter {}),
            );
            let data = vec![1, 2, 3];

            match m.send(data, |_| Ok(())) {
                Err(super::Error::EncryptData(_)) => (),
                x => panic!("Unexpected result: {:?}", x),
            }
        }
    }
}
