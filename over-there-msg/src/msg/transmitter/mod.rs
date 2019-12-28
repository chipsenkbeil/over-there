pub mod file;
pub mod tcp;
pub mod udp;

use super::Msg;
use over_there_transport::transmitter;
use over_there_transport::Transmitter;

#[derive(Debug)]
pub enum Error {
    EncodeMsg(rmp_serde::encode::Error),
    DecodeMsg(rmp_serde::decode::Error),
    SendData(transmitter::Error),
    RecvData(transmitter::Error),
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
    transmitter: Transmitter,
}

impl MsgTransmitter {
    pub fn new(transmitter: Transmitter) -> Self {
        MsgTransmitter { transmitter }
    }

    pub fn send(
        &self,
        msg: Msg,
        send_handler: impl FnMut(Vec<u8>) -> Result<(), std::io::Error>,
    ) -> Result<(), Error> {
        let data = msg.to_vec().map_err(Error::EncodeMsg)?;
        self.transmitter
            .send(data, send_handler)
            .map_err(Error::SendData)
    }

    pub fn recv(
        &self,
        recv_handler: impl FnMut(&mut [u8]) -> Result<usize, std::io::Error>,
    ) -> Result<Option<Msg>, Error> {
        self.transmitter
            .recv(recv_handler)
            .map_err(Error::RecvData)?
            .map(|v| Msg::from_slice(&v))
            .transpose()
            .map_err(Error::DecodeMsg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::Request;

    #[test]
    fn send_should_fail_if_unable_to_send_data() {
        let m = MsgTransmitter::new(Transmitter::with_transmission_size(100));
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
        let m = MsgTransmitter::new(Transmitter::with_transmission_size(100));
        let msg = Msg::new_request(Request::HeartbeatRequest);

        assert_eq!(m.send(msg, |_| Ok(())).is_ok(), true);
    }

    #[test]
    fn recv_should_fail_if_unable_to_receive_data() {
        let m = MsgTransmitter::new(Transmitter::with_transmission_size(100));

        match m.recv(|_| Err(std::io::Error::from(std::io::ErrorKind::Other))) {
            Err(Error::RecvData(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn recv_should_fail_if_unable_to_convert_complete_data_to_msg() {
        let m = MsgTransmitter::new(Transmitter::with_transmission_size(100));

        // Construct a data representation that is valid to read
        // but is not a msg
        let data: [u8; 100] = {
            let mut tmp = [0; 100];
            m.transmitter
                .send(vec![1, 2, 3], |msg_data| {
                    tmp[..msg_data.len()].clone_from_slice(&msg_data);
                    Ok(())
                })
                .unwrap();
            tmp
        };

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
        let m = MsgTransmitter::new(Transmitter::with_transmission_size(100));
        let msg = Msg::new_request(Request::HeartbeatRequest);

        // Construct a data representation for our message
        let data: [u8; 100] = {
            let mut tmp = [0; 100];
            m.transmitter
                .send(msg.to_vec().unwrap(), |msg_data| {
                    tmp[..msg_data.len()].clone_from_slice(&msg_data);
                    Ok(())
                })
                .unwrap();
            tmp
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
