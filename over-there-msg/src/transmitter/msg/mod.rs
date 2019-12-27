pub mod tcp;
pub mod udp;

use super::data::{self, DataTransmitter};
use crate::msg::Msg;

#[derive(Debug)]
pub enum Error {
    EncodeMsg(rmp_serde::encode::Error),
    DecodeMsg(rmp_serde::decode::Error),
    SendData(data::Error),
    RecvData(data::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &*self {
            Error::EncodeMsg(error) => write!(f, "Failed to encode message: {:?}", error),
            Error::DecodeMsg(error) => write!(f, "Failed to decode message: {:?}", error),
            Error::SendData(error) => write!(f, "Failed to send data: {:?}", error),
            Error::RecvData(error) => write!(f, "Failed to receive data: {:?}", error),
        }
    }
}

impl std::error::Error for Error {}

pub struct MsgTransmitter {
    data_transmitter: DataTransmitter,
}

impl MsgTransmitter {
    pub fn new(data_transmitter: DataTransmitter) -> Self {
        MsgTransmitter { data_transmitter }
    }

    pub fn send(
        &self,
        msg: Msg,
        send_handler: impl FnMut(Vec<u8>) -> Result<(), std::io::Error>,
    ) -> Result<(), Error> {
        let data = msg.to_vec().map_err(Error::EncodeMsg)?;
        self.data_transmitter
            .send(data, send_handler)
            .map_err(Error::SendData)
    }

    pub fn recv(
        &self,
        recv_handler: impl FnMut(&mut [u8]) -> Result<usize, std::io::Error>,
    ) -> Result<Option<Msg>, Error> {
        self.data_transmitter
            .recv(recv_handler)
            .map_err(Error::RecvData)?
            .map(|v| Msg::from_vec(&v))
            .transpose()
            .map_err(Error::DecodeMsg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::Request;
    use over_there_transport::{Disassembler, Packet};

    #[test]
    fn send_should_fail_if_unable_to_send_data() {
        let m = MsgTransmitter::new(DataTransmitter::new(100));
        let msg = Msg::new_request(Request::HeartbeatRequest);

        match m.send(msg, |_| {
            Err(std::io::Error::from(std::io::ErrorKind::Other))
        }) {
            Err(Error::SendData(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn send_should_succeed_if_able_to_send_msg() {
        let m = MsgTransmitter::new(DataTransmitter::new(100));
        let msg = Msg::new_request(Request::HeartbeatRequest);

        assert_eq!(m.send(msg, |_| Ok(())).is_ok(), true);
    }

    #[test]
    fn recv_should_fail_if_unable_to_receive_data() {
        let m = MsgTransmitter::new(DataTransmitter::new(100));

        match m.recv(|_| Err(std::io::Error::from(std::io::ErrorKind::Other))) {
            Err(Error::RecvData(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn recv_should_fail_if_unable_to_convert_complete_data_to_msg() {
        let m = MsgTransmitter::new(DataTransmitter::new(100));

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
    fn recv_should_succeed_if_able_to_receive_msg() {
        let m = MsgTransmitter::new(DataTransmitter::new(100));
        let msg = Msg::new_request(Request::HeartbeatRequest);

        // Convert msg into one large packet that we'll send for our test
        let data = {
            let data = msg.to_vec().unwrap();
            let psize = Packet::metadata_size() + data.len() as u32;
            let p = &Disassembler::make_packets_from_data(0, data, psize).unwrap()[0];
            p.to_vec().unwrap()
        };

        match m.recv(|buf| {
            let l = data.len();
            buf[..l].clone_from_slice(&data);
            Ok(l)
        }) {
            Ok(_) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }
}
