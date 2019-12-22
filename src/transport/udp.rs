use super::data::{assembler::Assembler, disassembler, Packet};
use super::MsgAndAddr;
use crate::msg::Msg;
use rand::random;
use std::cell::RefCell;
use std::error::Error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
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

    // 60001–61000
    pub const DEFAULT_PORT_RANGE: std::ops::Range<u16> = (60001..61000);

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

    /// Creates a new instance of a UDP transport layer using default settings
    pub fn local() -> Result<Self, std::io::Error> {
        UDP::new(
            IpAddr::from(Ipv4Addr::LOCALHOST),
            UDP::DEFAULT_PORT_RANGE.collect(),
            UDP::MAX_IPV4_DATAGRAM_SIZE as u32,
        )
    }

    pub fn addr(&self) -> Result<SocketAddr, std::io::Error> {
        self.sock.local_addr()
    }

    pub fn ip(&self) -> Result<IpAddr, std::io::Error> {
        let addr = self.addr()?;
        Ok(addr.ip())
    }

    pub fn port(&self) -> Result<u16, std::io::Error> {
        let addr = self.addr()?;
        Ok(addr.port())
    }
}

impl super::Transport for UDP {
    fn send(&self, msg_and_addr: MsgAndAddr) -> Result<(), Box<dyn Error>> {
        let MsgAndAddr(msg, addr) = msg_and_addr;
        let data = msg.to_vec()?;

        // Split message into multiple packets
        let id: u32 = random();
        let packets = disassembler::make_packets_from_data(id, data, self.max_data_per_packet)?;

        // For each packet, serialize and send to specific address
        for packet in packets.iter() {
            let packet_data = packet.to_vec()?;
            self.sock.send_to(&packet_data, addr)?;
        }

        Ok(())
    }

    fn recv(&self) -> Result<Option<MsgAndAddr>, Box<dyn Error>> {
        let mut buf = self.buffer.borrow_mut();
        let (_, src) = self.sock.recv_from(&mut buf[..])?;

        // Process the packet received from the UDP socket
        let p = Packet::from_slice(&buf[..])?;
        let p_id = p.get_id();

        // Grab a reference to our cache of packet assemblers that we will use; also drop any
        // expired assemblers
        let mut map = self.cache.borrow_mut();
        // TODO: Remove expired

        // Retrieve the assembler associated with the packet or
        // create a new instance
        let assembler = match map.get_mut(&p_id) {
            Some(a) => a,
            None => {
                let d = Duration::new(UDP::MAX_CACHE_DURATION_SECS, 0);
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
            let msg_and_addr = MsgAndAddr(m, src);

            // We also want to drop the assembler at this point
            map.remove(&p_id);

            Ok(Some(msg_and_addr))
        } else {
            Ok(None)
        }
    }
}
