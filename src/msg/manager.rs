use crate::msg::Msg;
use crate::transport;
use crate::transport::data::{assembler::Assembler, disassembler, Packet};
use log::debug;
use rand::random;
use std::cell::RefCell;
use std::time::Duration;
use ttl_cache::TtlCache;

#[derive(Debug)]
pub enum Error {
    FailedToEncodeMsg(rmp_serde::encode::Error),
    FailedToDecodeMsg(rmp_serde::decode::Error),
    FailedToEncodePacket(rmp_serde::encode::Error),
    FailedToDecodePacket(rmp_serde::decode::Error),
    FailedToAssembleData(transport::data::assembler::Error),
    FailedToDisassembleData(transport::data::disassembler::Error),
    FailedToSend(Box<dyn std::error::Error>),
    FailedToRecv(Box<dyn std::error::Error>),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &*self {
            Error::FailedToEncodeMsg(msg) => write!(f, "Failed to encode message: {:?}", msg),
            Error::FailedToDecodeMsg(error) => write!(f, "Failed to decode message: {:?}", error),
            Error::FailedToEncodePacket(packet) => {
                write!(f, "Failed to encode packet: {:?}", packet)
            }
            Error::FailedToDecodePacket(packet) => {
                write!(f, "Failed to decode packet: {:?}", packet)
            }
            Error::FailedToAssembleData(error) => write!(f, "Failed to assemble data: {:?}", error),
            Error::FailedToDisassembleData(error) => {
                write!(f, "Failed to disassemble data: {:?}", error)
            }
            Error::FailedToSend(error) => write!(f, "Failed to send data: {:?}", error),
            Error::FailedToRecv(error) => write!(f, "Failed to receive data: {:?}", error),
        }
    }
}

impl std::error::Error for Error {}

pub struct MsgManager {
    /// Maximum size allowed for a packet
    max_data_per_packet: u32,

    /// Cache of packets belonging to a group that has not been completed
    cache: RefCell<TtlCache<u32, Assembler>>,

    /// Buffer to contain bytes for temporary storage
    /// NOTE: Cannot use static array due to type constraints
    buffer: RefCell<Box<[u8]>>,
}

impl MsgManager {
    const MAX_CACHE_SIZE: usize = 1500;
    const MAX_CACHE_DURATION_SECS: u64 = 60 * 5;

    pub fn new(max_data_per_packet: u32) -> Self {
        MsgManager {
            max_data_per_packet,
            cache: RefCell::new(TtlCache::new(Self::MAX_CACHE_SIZE)),
            buffer: RefCell::new(vec![0; max_data_per_packet as usize].into_boxed_slice()),
        }
    }

    pub fn send(
        &self,
        msg: Msg,
        mut send_handler: impl FnMut(Vec<u8>) -> Result<(), Box<dyn std::error::Error>>,
    ) -> Result<(), Error> {
        let data = msg.to_vec().map_err(Error::FailedToEncodeMsg)?;

        // Split message into multiple packets
        let id: u32 = random();
        let packets = disassembler::make_packets_from_data(id, data, self.max_data_per_packet)
            .map_err(Error::FailedToDisassembleData)?;

        // For each packet, serialize and send to specific address
        for packet in packets.iter() {
            let packet_data = packet.to_vec().map_err(Error::FailedToEncodePacket)?;
            send_handler(packet_data).map_err(Error::FailedToSend)?;
        }

        Ok(())
    }

    pub fn recv(
        &self,
        mut recv_handler: impl FnMut(&mut [u8]) -> Result<usize, Box<dyn std::error::Error>>,
    ) -> Result<Option<Msg>, Error> {
        let mut buf = self.buffer.borrow_mut();
        let _bsize = recv_handler(&mut buf).map_err(Error::FailedToRecv)?;

        // Process the packet received from the UDP socket
        let p = Packet::from_slice(&buf).map_err(Error::FailedToDecodePacket)?;
        let p_id = p.get_id();
        debug!(
            "Packet [id: {} | index: {} | is_last: {}]",
            p_id,
            p.get_index(),
            p.is_last()
        );

        // Grab a reference to our cache of packet assemblers that we will use; also drop any
        // expired assemblers
        let mut map = self.cache.borrow_mut();
        // TODO: Remove expired

        // Retrieve the assembler associated with the packet or
        // create a new instance
        let assembler = match map.get_mut(&p_id) {
            Some(a) => a,
            None => {
                let d = Duration::new(Self::MAX_CACHE_DURATION_SECS, 0);
                map.insert(p.get_id(), Assembler::new(), d);
                map.get_mut(&p.get_id()).unwrap()
            }
        };

        // Bubble up the error; we don't care about the success
        assembler
            .add_packet(p)
            .map_err(Error::FailedToAssembleData)?;

        // Determine if time to assemble message
        if assembler.verify() {
            let data = assembler.assemble().map_err(Error::FailedToAssembleData)?;
            let m = Msg::from_vec(&data).map_err(Error::FailedToDecodeMsg)?;
            debug!("New message: {:?}", m);

            // We also want to drop the assembler at this point
            map.remove(&p_id);

            Ok(Some(m))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::{Msg, Request};
    use crate::transport::data::{assembler, disassembler};

    #[test]
    fn send_should_fail_if_unable_to_convert_bytes_to_packets() {
        // Produce a message manager with a "bytes per packet" that is too
        // low, causing the process to fail
        let m = MsgManager::new(0);
        let msg = Msg::from_request(0, vec![], Request::HeartbeatRequest);

        match m.send(msg, |_| Ok(())) {
            Err(Error::FailedToDisassembleData(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn send_should_fail_if_socket_fails_to_send_bytes() {
        let m = MsgManager::new(100);
        let msg = Msg::from_request(0, vec![], Request::HeartbeatRequest);

        match m.send(msg, |_| {
            Err(Box::new(std::io::Error::from(std::io::ErrorKind::Other)))
        }) {
            Err(Error::FailedToSend(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn send_should_return_okay_if_successfully_sent_message() {
        let m = MsgManager::new(100);
        let msg = Msg::from_request(0, vec![], Request::HeartbeatRequest);

        let result = m.send(msg, |_| Ok(()));
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    fn recv_should_fail_if_socket_fails_to_get_bytes() {
        let m = MsgManager::new(100);

        match m.recv(|_| Err(Box::new(std::io::Error::from(std::io::ErrorKind::Other)))) {
            Err(Error::FailedToRecv(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn recv_should_fail_if_unable_to_convert_bytes_to_packet() {
        let m = MsgManager::new(100);

        // Force buffer to have a couple of early zeros, which is not
        // valid data when decoding
        match m.recv(|buf| {
            buf[0] = 0;
            buf[1] = 0;
            buf[2] = 0;
            Ok(buf.len())
        }) {
            Err(Error::FailedToDecodePacket(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn recv_should_fail_if_unable_to_add_packet_to_assembler() {
        let m = MsgManager::new(100);

        // Make several packets so that we don't send a single and last
        // packet, which would remove itself from the cache and allow
        // us to re-add a packet with the same id & index
        let p = &disassembler::make_packets_from_data(
            0,
            Msg::from_request(0, vec![], Request::HeartbeatRequest)
                .to_vec()
                .unwrap(),
            Packet::metadata_size() + 1,
        )
        .unwrap()[0];
        assert_eq!(
            m.recv(|buf| {
                let data = p.to_vec()?;
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
            let data = p.to_vec()?;
            let l = data.len();
            buf[..l].clone_from_slice(&data);
            Ok(l)
        }) {
            Err(Error::FailedToAssembleData(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn recv_should_fail_if_unable_to_convert_complete_data_to_message() {
        let m = MsgManager::new(100);

        // Provide a valid packet, but one that does not form a message
        let p =
            &disassembler::make_packets_from_data(0, vec![1, 2, 3], Packet::metadata_size() + 3)
                .unwrap()[0];
        match m.recv(|buf| {
            let data = p.to_vec()?;
            let l = data.len();
            buf[..l].clone_from_slice(&data);
            Ok(l)
        }) {
            Err(Error::FailedToDecodeMsg(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn recv_should_return_none_if_received_packet_does_not_complete_message() {
        let m = MsgManager::new(100);

        // Make several packets so that we don't send a single and last
        // packet, which would result in a complete message
        let p = &disassembler::make_packets_from_data(
            0,
            Msg::from_request(0, vec![], Request::HeartbeatRequest)
                .to_vec()
                .unwrap(),
            Packet::metadata_size() + 1,
        )
        .unwrap()[0];
        match m.recv(|buf| {
            let data = p.to_vec()?;
            let l = data.len();
            buf[..l].clone_from_slice(&data);
            Ok(l)
        }) {
            Ok(None) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn recv_should_return_some_message_if_received_packet_does_complete_message() {
        let m = MsgManager::new(100);
        let msg = Msg::from_request(0, vec![], Request::HeartbeatRequest);

        // Make one large packet so we can complete a message
        let p = &disassembler::make_packets_from_data(0, msg.to_vec().unwrap(), 100).unwrap()[0];
        match m.recv(|buf| {
            let data = p.to_vec()?;
            let l = data.len();
            buf[..l].clone_from_slice(&data);
            Ok(l)
        }) {
            Ok(Some(recv_msg)) => {
                assert_eq!(
                    recv_msg.to_vec().unwrap(),
                    msg.to_vec().unwrap(),
                    "Received unexpected message: {:?}",
                    recv_msg
                );
            }
            x => panic!("Unexpected result: {:?}", x),
        }
    }
}
