use crate::msg::Msg;
use log::debug;
use over_there_transport::{assembler, disassembler, Assembler, Disassembler, Packet};
use rand::random;
use std::cell::RefCell;
use std::time::Duration;
use ttl_cache::TtlCache;

#[derive(Debug)]
pub enum Error {
    EncodeMsg(rmp_serde::encode::Error),
    DecodeMsg(rmp_serde::decode::Error),
    EncodePacket(rmp_serde::encode::Error),
    DecodePacket(rmp_serde::decode::Error),
    AssembleData(assembler::Error),
    DisassembleData(disassembler::Error),
    SendBytes(std::io::Error),
    RecvBytes(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &*self {
            Error::EncodeMsg(msg) => write!(f, "Failed to encode message: {:?}", msg),
            Error::DecodeMsg(error) => write!(f, "Failed to decode message: {:?}", error),
            Error::EncodePacket(packet) => write!(f, "Failed to encode packet: {:?}", packet),
            Error::DecodePacket(packet) => write!(f, "Failed to decode packet: {:?}", packet),
            Error::AssembleData(error) => write!(f, "Failed to assemble data: {:?}", error),
            Error::DisassembleData(error) => write!(f, "Failed to disassemble data: {:?}", error),
            Error::SendBytes(error) => write!(f, "Failed to send bytes: {:?}", error),
            Error::RecvBytes(error) => write!(f, "Failed to receive bytes: {:?}", error),
        }
    }
}

impl std::error::Error for Error {}

pub struct MsgTransmitter {
    /// Maximum size allowed for a packet
    max_data_per_packet: u32,

    /// Cache of packets belonging to a group that has not been completed
    cache: RefCell<TtlCache<u32, Assembler>>,

    /// Buffer to contain bytes for temporary storage
    /// NOTE: Cannot use static array due to type constraints
    buffer: RefCell<Box<[u8]>>,
}

impl MsgTransmitter {
    const MAX_CACHE_SIZE: usize = 1500;
    const MAX_CACHE_DURATION_SECS: u64 = 60 * 5;

    pub fn new(max_data_per_packet: u32) -> Self {
        MsgTransmitter {
            max_data_per_packet,
            cache: RefCell::new(TtlCache::new(Self::MAX_CACHE_SIZE)),
            buffer: RefCell::new(vec![0; max_data_per_packet as usize].into_boxed_slice()),
        }
    }

    pub fn send(
        &self,
        msg: Msg,
        mut send_handler: impl FnMut(Vec<u8>) -> Result<(), std::io::Error>,
    ) -> Result<(), Error> {
        let data = msg.to_vec().map_err(Error::EncodeMsg)?;

        // Split message into multiple packets
        let id: u32 = random();
        let packets = Disassembler::make_packets_from_data(id, data, self.max_data_per_packet)
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
    ) -> Result<Option<Msg>, Error> {
        let mut buf = self.buffer.borrow_mut();
        let _bsize = recv_handler(&mut buf).map_err(Error::RecvBytes)?;

        // Process the packet received from the UDP socket
        let p = Packet::from_slice(&buf).map_err(Error::DecodePacket)?;
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
        // TODO: Remove expired

        // Retrieve the assembler associated with the packet or
        // create a new instance
        let assembler = match map.get_mut(&p_id) {
            Some(a) => a,
            None => {
                let d = Duration::new(Self::MAX_CACHE_DURATION_SECS, 0);
                map.insert(p.id(), Assembler::new(), d);
                map.get_mut(&p.id()).unwrap()
            }
        };

        // Bubble up the error; we don't care about the success
        assembler.add_packet(p).map_err(Error::AssembleData)?;

        // Determine if time to assemble message
        if assembler.verify() {
            let data = assembler.assemble().map_err(Error::AssembleData)?;
            let m = Msg::from_vec(&data).map_err(Error::DecodeMsg)?;
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

    #[test]
    fn send_should_fail_if_unable_to_convert_bytes_to_packets() {
        // Produce a message manager with a "bytes per packet" that is too
        // low, causing the process to fail
        let m = MsgTransmitter::new(0);
        let msg = Msg::new_request(Request::HeartbeatRequest);

        match m.send(msg, |_| Ok(())) {
            Err(Error::DisassembleData(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn send_should_fail_if_socket_fails_to_send_bytes() {
        let m = MsgTransmitter::new(100);
        let msg = Msg::new_request(Request::HeartbeatRequest);

        match m.send(msg, |_| {
            Err(std::io::Error::from(std::io::ErrorKind::Other))
        }) {
            Err(Error::SendBytes(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn send_should_return_okay_if_successfully_sent_message() {
        let m = MsgTransmitter::new(100);
        let msg = Msg::new_request(Request::HeartbeatRequest);

        let result = m.send(msg, |_| Ok(()));
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    fn recv_should_fail_if_socket_fails_to_get_bytes() {
        let m = MsgTransmitter::new(100);

        match m.recv(|_| Err(std::io::Error::from(std::io::ErrorKind::Other))) {
            Err(Error::RecvBytes(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn recv_should_fail_if_unable_to_convert_bytes_to_packet() {
        let m = MsgTransmitter::new(100);

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
        let m = MsgTransmitter::new(100);

        // Make several packets so that we don't send a single and last
        // packet, which would remove itself from the cache and allow
        // us to re-add a packet with the same id & index
        let p = &Disassembler::make_packets_from_data(
            0,
            Msg::new_request(Request::HeartbeatRequest)
                .to_vec()
                .unwrap(),
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
    fn recv_should_fail_if_unable_to_convert_complete_data_to_message() {
        let m = MsgTransmitter::new(100);

        // Provide a valid packet, but one that does not form a message
        let p =
            &Disassembler::make_packets_from_data(0, vec![1, 2, 3], Packet::metadata_size() + 3)
                .unwrap()[0];

        let data = p.to_vec().unwrap();
        match m.recv(|buf| {
            let l = data.len();
            buf[..l].clone_from_slice(&data);
            Ok(l)
        }) {
            Err(Error::DecodeMsg(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn recv_should_return_none_if_received_packet_does_not_complete_message() {
        let m = MsgTransmitter::new(100);

        // Make several packets so that we don't send a single and last
        // packet, which would result in a complete message
        let p = &Disassembler::make_packets_from_data(
            0,
            Msg::new_request(Request::HeartbeatRequest)
                .to_vec()
                .unwrap(),
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
    fn recv_should_return_some_message_if_received_packet_does_complete_message() {
        let m = MsgTransmitter::new(100);
        let msg = Msg::new_request(Request::HeartbeatRequest);

        // Make one large packet so we can complete a message
        let p = &Disassembler::make_packets_from_data(0, msg.to_vec().unwrap(), 100).unwrap()[0];
        let data = p.to_vec().unwrap();
        match m.recv(|buf| {
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
