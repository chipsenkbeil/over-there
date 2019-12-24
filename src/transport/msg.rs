use super::super::msg::Msg;
use super::data::{assembler::Assembler, disassembler, Packet};
use super::net::NetworkTransport;
use log::debug;
use rand::random;
use std::cell::RefCell;
use std::error::Error;
use std::net::SocketAddr;
use std::time::Duration;
use ttl_cache::TtlCache;

pub struct MsgTransport<T: NetworkTransport> {
    /// Means of sending actual data
    transport: T,

    /// Maximum size allowed for a packet
    max_data_per_packet: u32,

    /// Cache of packets belonging to a group that has not been completed
    cache: RefCell<TtlCache<u32, Assembler>>,

    /// Buffer to contain bytes for temporary storage
    /// NOTE: Cannot use static array due to type constraints
    buffer: RefCell<Box<[u8]>>,
}

impl<T: NetworkTransport> MsgTransport<T> {
    const MAX_CACHE_SIZE: usize = 1500;
    const MAX_CACHE_DURATION_SECS: u64 = 60 * 5;

    pub fn new(transport: T, max_data_per_packet: u32) -> Self {
        MsgTransport {
            transport,
            max_data_per_packet,
            cache: RefCell::new(TtlCache::new(Self::MAX_CACHE_SIZE)),
            buffer: RefCell::new(vec![0; max_data_per_packet as usize].into_boxed_slice()),
        }
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn send(&self, msg: Msg, addr: SocketAddr) -> Result<(), Box<dyn Error>> {
        let data = msg.to_vec()?;

        // Split message into multiple packets
        let id: u32 = random();
        let packets = disassembler::make_packets_from_data(id, data, self.max_data_per_packet)?;

        // For each packet, serialize and send to specific address
        for packet in packets.iter() {
            let packet_data = packet.to_vec()?;
            self.transport.send_data(packet_data, addr)?;
        }

        Ok(())
    }

    pub fn recv(&self) -> Result<Option<(Msg, SocketAddr)>, Box<dyn Error>> {
        let mut buf = self.buffer.borrow_mut();
        let (_bsize, src) = self.transport.recv_data(&mut buf)?;

        // Process the packet received from the UDP socket
        let p = Packet::from_slice(&buf)?;
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
        assembler.add_packet(p)?;

        // Determine if time to assemble message
        if assembler.verify() {
            let data = assembler.assemble()?;
            let m = Msg::from_vec(&data)?;
            debug!("New message: {:?}", m);

            // We also want to drop the assembler at this point
            map.remove(&p_id);

            Ok(Some((m, src)))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_should_fail_if_unable_to_convert_message_to_bytes() {
        panic!("TODO: Implement test");
    }

    #[test]
    fn send_should_fail_if_unable_to_convert_bytes_to_packets() {
        panic!("TODO: Implement test");
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
        panic!("TODO: Implement test");
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
