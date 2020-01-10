pub mod tcp;
pub mod udp;

use super::Msg;
use over_there_auth::Signer;
use over_there_crypto::Encrypter;
use over_there_derive::Error;
use over_there_transport::{Transmitter, TransmitterError};

#[derive(Debug, Error)]
pub enum MsgTransmitterError {
    EncodeMsg(rmp_serde::encode::Error),
    SendData(TransmitterError),
}

pub struct MsgTransmitter<'a, S, E>
where
    S: Signer,
    E: Encrypter,
{
    transmitter: &'a Transmitter<'a, S, E>,
}

impl<'a, S, E> MsgTransmitter<'a, S, E>
where
    S: Signer,
    E: Encrypter,
{
    pub fn new(transmitter: &'a Transmitter<'a, S, E>) -> Self {
        Self { transmitter }
    }

    pub fn send(
        &self,
        msg: Msg,
        send_handler: impl FnMut(&[u8]) -> Result<(), std::io::Error>,
    ) -> Result<(), MsgTransmitterError> {
        let data = msg.to_vec().map_err(MsgTransmitterError::EncodeMsg)?;
        self.transmitter
            .send(&data, send_handler)
            .map_err(MsgTransmitterError::SendData)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::content::Content;
    use over_there_auth::NoopAuthenticator;
    use over_there_crypto::NoopBicrypter;

    fn new_transmitter<'a>(
        transmission_size: usize,
    ) -> Transmitter<'a, NoopAuthenticator, NoopBicrypter> {
        Transmitter::new(transmission_size, &NoopAuthenticator, &NoopBicrypter)
    }

    #[test]
    fn send_should_fail_if_unable_to_send_data() {
        let t = new_transmitter(100);
        let m = MsgTransmitter::new(&t);
        let msg = Msg::from(Content::HeartbeatRequest);

        match m.send(msg, |_| {
            Err(std::io::Error::from(std::io::ErrorKind::Other))
        }) {
            Err(MsgTransmitterError::SendData(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn send_should_succeed_if_able_to_send_msg() {
        let t = new_transmitter(100);
        let m = MsgTransmitter::new(&t);
        let msg = Msg::from(Content::HeartbeatRequest);

        assert_eq!(m.send(msg, |_| Ok(())).is_ok(), true);
    }
}
