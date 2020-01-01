pub mod file;
pub mod tcp;
pub mod udp;

use super::Msg;
use over_there_derive::Error;
use over_there_transport::{Transmitter, TransmitterError};

#[derive(Debug, Error)]
pub enum MsgTransmitterError {
    EncodeMsg(rmp_serde::encode::Error),
    DecodeMsg(rmp_serde::decode::Error),
    SendData(TransmitterError),
    RecvData(TransmitterError),
}

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
    ) -> Result<(), MsgTransmitterError> {
        let data = msg.to_vec().map_err(MsgTransmitterError::EncodeMsg)?;
        self.transmitter
            .send(data, send_handler)
            .map_err(MsgTransmitterError::SendData)
    }

    pub fn recv(
        &self,
        recv_handler: impl FnMut(&mut [u8]) -> Result<usize, std::io::Error>,
    ) -> Result<Option<Msg>, MsgTransmitterError> {
        self.transmitter
            .recv(recv_handler)
            .map_err(MsgTransmitterError::RecvData)?
            .map(|v| Msg::from_slice(&v))
            .transpose()
            .map_err(MsgTransmitterError::DecodeMsg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::types::request::StandardRequest as Request;

    fn new_msg_transmitter(transmission_size: usize) -> MsgTransmitter {
        use over_there_crypto::NoopBicrypter;
        use std::time::Duration;
        let cache_capacity = 1500;
        let cache_duration = Duration::from_secs(5 * 60);
        let bicrypter = Box::new(NoopBicrypter::new());
        MsgTransmitter::new(Transmitter::new(
            transmission_size,
            cache_capacity,
            cache_duration,
            bicrypter,
        ))
    }

    #[test]
    fn send_should_fail_if_unable_to_send_data() {
        let m = new_msg_transmitter(100);
        let msg = Msg::from_content(Request::HeartbeatRequest);

        match m.send(msg, |_| {
            Err(std::io::Error::from(std::io::ErrorKind::Other))
        }) {
            Err(MsgTransmitterError::SendData(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn send_should_succeed_if_able_to_send_msg() {
        let m = new_msg_transmitter(100);
        let msg = Msg::from_content(Request::HeartbeatRequest);

        assert_eq!(m.send(msg, |_| Ok(())).is_ok(), true);
    }

    #[test]
    fn recv_should_fail_if_unable_to_receive_data() {
        let m = new_msg_transmitter(100);

        match m.recv(|_| Err(std::io::Error::from(std::io::ErrorKind::Other))) {
            Err(MsgTransmitterError::RecvData(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn recv_should_fail_if_unable_to_convert_complete_data_to_msg() {
        let m = new_msg_transmitter(100);

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
            Err(MsgTransmitterError::DecodeMsg(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn recv_should_succeed_if_able_to_receive_msg() {
        let m = new_msg_transmitter(100);
        let msg = Msg::from_content(Request::HeartbeatRequest);

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
