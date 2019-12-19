use super::msg::Msg;
use super::packet::Packet;
use std::error::Error;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use ttl_cache::TtlCache;

pub struct UDP {
    /// Internal socket used for communication
    sock: UdpSocket,

    /// Maximum size allowed for a packet
    /// 508 bytes for data; 508 = 576 - 60 (IP header) - 8 (udp header)
    max_size: usize,

    /// Cache of packets belonging to a group that has not been completed
    cache: TtlCache<u32, Vec<Packet>>,
}

impl UDP {
    // IPv4 :: 508 = 576 - 60 (IP header) - 8 (udp header)
    pub const MAX_IPV4_DATAGRAM_SIZE: usize = 508;

    // IPv6 :: 1212 = 1280 - 60 (IP header) - 8 (udp header)
    pub const MAX_IPV6_DATAGRAM_SIZE: usize = 1212;

    pub const MAX_CACHE_SIZE: usize = 1500;

    /// Creates a new instance of a UDP transport layer, binding to the
    /// specified host using the first open port in the list provided.
    pub fn new(
        host: IpAddr,
        port: Vec<u16>,
        cache_capacity: usize,
    ) -> Result<Self, std::io::Error> {
        let addr_candidates: Vec<SocketAddr> =
            port.iter().map(|p| SocketAddr::new(host, *p)).collect();
        let sock = UdpSocket::bind(&addr_candidates[..])?;
        let instance = UDP {
            sock,
            max_size: UDP::MAX_CACHE_SIZE,
            cache: TtlCache::new(cache_capacity),
        };
        Ok(instance)
    }
}

impl super::Transport for UDP {
    fn send(&self, msg: Msg) -> Result<(), Box<dyn Error>> {
        let data = msg.to_vec()?;

        // TODO: Split data into chunks if larger than our max size
        //       and add our metadata to a packet to send off
        let packets = Packet::data_to_multipart(data, Self::MAX_IPV4_DATAGRAM_SIZE);

        // For each packet, serialize and send to everyone
        for packet in packets.iter() {
            let packet_data = rmp_serde::to_vec(&packet)?;
            self.sock.send(&packet_data);
        }

        Ok(())
    }

    fn recv(&self) -> Result<Option<Msg>, Box<dyn Error>> {
        // TODO: Have a common buffer of max size? Recreate buffer each time?
        let mut buf = [0; Self::MAX_IPV4_DATAGRAM_SIZE];

        let (len, src) = self.sock.recv_from(&mut buf)?;
        let p: Packet = rmp_serde::from_read_ref(&buf[..])?;

        // if p.is_multipart() {
        //     // If this packet is part of a group, store it in our cache and
        //     // only put back together the message if we have the full collection
        //     Ok()
        // } else {
        //     // Otherwise, this is a single datagram that we can consume immediately
        //     let m = Msg::from_vec(&p.get_data())?;
        //     Ok(m)
        // }

        let m = Msg::from_vec(&p.get_data())?;
        Ok(Some(m))
    }
}
