use crate::msg::Msg;
use crate::transport;
use crate::transport::data::{assembler::Assembler, disassembler, Packet};
use log::debug;
use rand::random;
use std::cell::RefCell;
use std::time::Duration;
use ttl_cache::TtlCache;

#[derive(Debug)]
pub enum MsgManangerError {
    FailedToEncodeMsg(rmp_serde::encode::Error),
    FailedToDecodeMsg(rmp_serde::decode::Error),
    FailedToEncodePacket(rmp_serde::encode::Error),
    FailedToDecodePacket(rmp_serde::decode::Error),
    FailedToAssembleData(transport::data::assembler::Error),
    FailedToDisassembleData(transport::data::disassembler::Error),
    FailedToSend(Box<dyn std::error::Error>),
    FailedToRecv(Box<dyn std::error::Error>),
}

impl std::fmt::Display for MsgManangerError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &*self {
            MsgManangerError::FailedToEncodeMsg(msg) => {
                write!(f, "Failed to encode message: {:?}", msg)
            }
            MsgManangerError::FailedToDecodeMsg(error) => {
                write!(f, "Failed to decode message: {:?}", error)
            }
            MsgManangerError::FailedToEncodePacket(packet) => {
                write!(f, "Failed to encode packet: {:?}", packet)
            }
            MsgManangerError::FailedToDecodePacket(packet) => {
                write!(f, "Failed to decode packet: {:?}", packet)
            }
            MsgManangerError::FailedToAssembleData(error) => {
                write!(f, "Failed to assemble data: {:?}", error)
            }
            MsgManangerError::FailedToDisassembleData(error) => {
                write!(f, "Failed to disassemble data: {:?}", error)
            }
            MsgManangerError::FailedToSend(error) => write!(f, "Failed to send data: {:?}", error),
            MsgManangerError::FailedToRecv(error) => {
                write!(f, "Failed to receive data: {:?}", error)
            }
        }
    }
}

impl std::error::Error for MsgManangerError {}

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
    ) -> Result<(), MsgManangerError> {
        let data = msg.to_vec().map_err(MsgManangerError::FailedToEncodeMsg)?;

        // Split message into multiple packets
        let id: u32 = random();
        let packets = disassembler::make_packets_from_data(id, data, self.max_data_per_packet)
            .map_err(MsgManangerError::FailedToDisassembleData)?;

        // For each packet, serialize and send to specific address
        for packet in packets.iter() {
            let packet_data = packet
                .to_vec()
                .map_err(MsgManangerError::FailedToEncodePacket)?;
            send_handler(packet_data).map_err(MsgManangerError::FailedToSend)?;
        }

        Ok(())
    }

    pub fn recv(
        &self,
        mut recv_handler: impl FnMut(&mut [u8]) -> Result<usize, Box<dyn std::error::Error>>,
    ) -> Result<Option<Msg>, MsgManangerError> {
        let mut buf = self.buffer.borrow_mut();
        let _bsize = recv_handler(&mut buf).map_err(MsgManangerError::FailedToRecv)?;

        // Process the packet received from the UDP socket
        let p = Packet::from_slice(&buf).map_err(MsgManangerError::FailedToDecodePacket)?;
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
            .map_err(MsgManangerError::FailedToAssembleData)?;

        // Determine if time to assemble message
        if assembler.verify() {
            let data = assembler
                .assemble()
                .map_err(MsgManangerError::FailedToAssembleData)?;
            let m = Msg::from_vec(&data).map_err(MsgManangerError::FailedToDecodeMsg)?;
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
    use crate::msg::Request;
    use crate::transport::data::{assembler, disassembler};

    #[test]
    fn send_should_fail_if_unable_to_convert_message_to_bytes() {
        panic!("TODO: Implement test");
    }

    #[test]
    fn send_should_fail_if_unable_to_convert_bytes_to_packets() {
        // Produce a message manager with a "bytes per packet" that is too
        // low, causing the process to fail
        let m = MsgManager::new(0);
        let msg = Msg::from_request(0, vec![], Request::HeartbeatRequest);

        // NOTE: Cannot pattern match or evaluate the specifics of the error
        let result = m.send(msg, |_| Ok(()));
        assert_eq!(result.is_err(), true,);
    }

    #[test]
    fn send_should_fail_if_unable_to_convert_packet_to_bytes() {
        panic!("TODO: Implement test");
    }

    #[test]
    fn send_should_fail_if_socket_fails_to_send_bytes() {
        panic!("TODO: Implement test");
    }

    #[test]
    fn send_should_return_okay_if_successfully_sent_message() {
        let m = MsgManager::new(9999);
        let msg = Msg::from_request(0, vec![], Request::HeartbeatRequest);

        // NOTE: Cannot pattern match or evaluate the specifics of the error
        let result = m.send(msg, |_| Ok(()));
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    fn recv_should_fail_if_socket_fails_to_get_bytes() {
        panic!("TODO: Implement test");
    }

    #[test]
    fn recv_should_fail_if_unable_to_convert_bytes_to_packet() {
        panic!("TODO: Implement test");
    }

    #[test]
    fn recv_should_fail_if_unable_to_add_packet_to_assembler() {
        panic!("TODO: Implement test");
    }

    #[test]
    fn recv_should_fail_if_unable_to_assemble_packet_data() {
        panic!("TODO: Implement test");
    }

    #[test]
    fn recv_should_fail_if_unable_to_convert_complete_data_to_message() {
        panic!("TODO: Implement test");
    }

    #[test]
    fn recv_should_return_none_if_received_packet_does_not_complete_message() {
        panic!("TODO: Implement test");
    }

    #[test]
    fn recv_should_return_some_message_if_received_packet_does_complete_message() {
        panic!("TODO: Implement test");
    }
}
