use std::error::Error;
use std::net::SocketAddr;
use std::time::Duration;

pub fn parse_duration(s: &str) -> Result<Duration, Box<dyn Error>> {
    let secs: f64 = s.parse()?;
    Ok(Duration::from_secs_f64(secs))
}

pub fn parse_socket_addr(s: &str) -> Result<SocketAddr, Box<dyn Error>> {
    let addr = s.parse()?;
    Ok(addr)
}
