use std::io::{self, Read, Result, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};

/// Maximum Transmission Unit for Ethernet in bytes
pub const MTU_ETHERNET_SIZE: usize = 1500;

/// Maximum Transmission Unit for Dialup in bytes
pub const MTU_DIALUP_SIZE: usize = 576;

/// 8080-8099
pub const DEFAULT_PORT_RANGE: std::ops::Range<u16> = (8080..8099);

pub fn bind(host: IpAddr, port: Vec<u16>) -> Result<TcpListener> {
    let addr_candidates: Vec<SocketAddr> = port.iter().map(|p| SocketAddr::new(host, *p)).collect();
    TcpListener::bind(&addr_candidates[..])
}

pub fn local() -> Result<TcpListener> {
    bind(
        IpAddr::from(Ipv4Addr::LOCALHOST),
        DEFAULT_PORT_RANGE.collect(),
    )
}

/// Produces a new function to send data using a tcp stream
/// NOTE: This consumes the stream, so a clone should attempt to be made
pub fn new_send_func(mut stream: TcpStream) -> impl FnMut(&[u8]) -> Result<()> {
    move |data| {
        // TODO: Support sending remaining bytes in loop? Would need to
        //       support collecting bytes for a packet in multiple receives,
        //       which means we'd need a start and stop indicator of some
        //       kind that is a single byte. Seems too complicated, so
        //       easier to fail and give a reason if we don't send all
        //       of the bytes in one go. It's one of the reasons we made
        //       packets of a guaranteed max size.
        let size = stream.write(&data)?;
        if size < data.len() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Only sent {} bytes out of {}", size, data.len()),
            ));
        }
        Ok(())
    }
}

/// Produces a new function to receive data using a tcp stream
/// NOTE: This consumes the stream, so a clone should attempt to be made
pub fn new_recv_func(
    mut stream: TcpStream,
    addr: SocketAddr,
) -> impl FnMut(&mut [u8]) -> Result<(usize, SocketAddr)> {
    move |data| stream.read(data).map(|s| (s, addr))
}
