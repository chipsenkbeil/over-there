use super::data::assembler::Assembler;
use super::data::disassembler;
use super::data::Packet;
use crate::msg::Msg;
use std::cell::RefCell;
use std::error::Error;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::time::Duration;
use ttl_cache::TtlCache;

pub struct UDP {
    /// Internal socket used for communication
    sock: UdpSocket,

    /// Maximum size allowed for a packet
    max_data_per_packet: u32,

    /// Cache of packets belonging to a group that has not been completed
    cache: RefCell<TtlCache<u32, Assembler>>,

    /// Buffer to contain bytes for temporary storage
    buffer: RefCell<[u8; Self::MAX_IPV4_DATAGRAM_SIZE]>,
}

impl UDP {
    // IPv4 :: 508 = 576 - 60 (IP header) - 8 (udp header)
    pub const MAX_IPV4_DATAGRAM_SIZE: usize = 508;

    // IPv6 :: 1212 = 1280 - 60 (IP header) - 8 (udp header)
    pub const MAX_IPV6_DATAGRAM_SIZE: usize = 1212;

    pub const MAX_CACHE_SIZE: usize = 1500;
    pub const MAX_CACHE_DURATION_SECS: u64 = 60 * 5;

    /// Creates a new instance of a UDP transport layer, binding to the
    /// specified host using the first open port in the list provided.
    pub fn new(
        host: IpAddr,
        port: Vec<u16>,
        max_data_per_packet: u32,
    ) -> Result<Self, std::io::Error> {
        let addr_candidates: Vec<SocketAddr> =
            port.iter().map(|p| SocketAddr::new(host, *p)).collect();
        let sock = UdpSocket::bind(&addr_candidates[..])?;
        let instance = UDP {
            sock,
            max_data_per_packet,
            cache: RefCell::new(TtlCache::new(Self::MAX_CACHE_SIZE)),
            buffer: RefCell::new([0; Self::MAX_IPV4_DATAGRAM_SIZE]),
        };
        Ok(instance)
    }
}

impl super::Transport for UDP {
    fn send(&self, msg: Msg) -> Result<(), Box<dyn Error>> {
        let data = msg.to_vec()?;

        // TODO: Create unique id for group of packets
        let id = 0;
        let packets = disassembler::make_packets_from_data(id, data, self.max_data_per_packet);

        // For each packet, serialize and send to everyone
        for packet in packets.iter() {
            let packet_data = rmp_serde::to_vec(&packet)?;
            self.sock.send(&packet_data)?;
        }

        Ok(())
    }

    fn recv(&self) -> Result<Option<Msg>, Box<dyn Error>> {
        let mut buf = self.buffer.borrow_mut();
        let (_, src) = self.sock.recv_from(&mut buf[..])?;
        let p: Packet = rmp_serde::from_read_ref(&buf[..])?;

        // Retrieve the assembler associated with the packet or
        // create a new instance
        let mut map = self.cache.borrow_mut();
        let mut assembler = match map.get_mut(&p.get_id()) {
            Some(a) => a,
            None => {
                let d = Duration::new(UDP::MAX_CACHE_DURATION_SECS, 0);
                map.insert(p.get_id(), Assembler::new(), d);
                map.get_mut(&p.get_id()).unwrap()
            }
        };

        assembler.add_packet(p);
        assembler.assemble();
        assembler.assemble();
        Ok(None)

        // let m = Msg::from_vec(&p.get_data())?;
    }
}
