use over_there_utils::{self as utils, DelimiterReader, DelimiterWriter};
use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr, TcpListener, TcpStream};

/// Maximum Transmission Unit for Ethernet in bytes
pub const MTU_ETHERNET_SIZE: usize = 1500;

/// Maximum Transmission Unit for Dialup in bytes
pub const MTU_DIALUP_SIZE: usize = 576;

pub fn bind(host: IpAddr, port: Vec<u16>) -> io::Result<TcpListener> {
    let addr_candidates = super::make_addr_list(host, port);
    TcpListener::bind(&addr_candidates[..])
}

pub fn local() -> io::Result<TcpListener> {
    bind(
        IpAddr::from(Ipv4Addr::LOCALHOST),
        super::IANA_EPHEMERAL_PORT_RANGE.collect(),
    )
}

/// Represents a buffered TCP stream using delimiters to separate data sent
/// via writing and being read to ensure that messages are received properly
/// without capturing other bytes
pub struct BufTcpStream {
    pub inner: TcpStream,
    pub(crate) input: DelimiterReader<TcpStream>,
    output: DelimiterWriter<TcpStream>,
}

impl BufTcpStream {
    pub fn new_with_delimiter(
        stream: TcpStream,
        max_data_size: usize,
        delimiter: &[u8],
    ) -> io::Result<Self> {
        let input =
            DelimiterReader::new_with_delimiter(stream.try_clone()?, max_data_size, delimiter);
        let output = DelimiterWriter::new_with_delimiter(stream.try_clone()?, delimiter);

        Ok(Self {
            inner: stream,
            input,
            output,
        })
    }

    pub fn new(stream: TcpStream, max_data_size: usize) -> io::Result<Self> {
        Self::new_with_delimiter(stream, max_data_size, utils::DEFAULT_DELIMITER)
    }
}

impl Read for BufTcpStream {
    fn read(&mut self, data: &mut [u8]) -> io::Result<usize> {
        self.input.read(data)
    }
}

impl Write for BufTcpStream {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.output.write(data)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.output.flush()
    }
}
