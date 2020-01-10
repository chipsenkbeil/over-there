pub mod tcp;
pub mod udp;

use super::Msg;
use over_there_auth::Verifier;
use over_there_crypto::Decrypter;
use over_there_derive::Error;
use over_there_transport::{Receiver, ReceiverError};

#[derive(Debug, Error)]
pub enum MsgReceiverError {
    DecodeMsg(rmp_serde::decode::Error),
    RecvData(ReceiverError),
}

pub struct MsgReceiver<'a, V, D>
where
    V: Verifier,
    D: Decrypter,
{
    receiver: &'a Receiver<'a, V, D>,
}

impl<'a, V, D> MsgReceiver<'a, V, D>
where
    V: Verifier,
    D: Decrypter,
{
    pub fn new(receiver: &'a Receiver<'a, V, D>) -> Self {
        Self { receiver }
    }

    pub fn recv(
        &self,
        recv_handler: impl FnMut(&mut [u8]) -> Result<usize, std::io::Error>,
    ) -> Result<Option<Msg>, MsgReceiverError> {
        self.receiver
            .recv(recv_handler)
            .map_err(MsgReceiverError::RecvData)?
            .map(|v| Msg::from_slice(&v))
            .transpose()
            .map_err(MsgReceiverError::DecodeMsg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::content::Content;
    use over_there_auth::NoopAuthenticator;
    use over_there_crypto::NoopBicrypter;
    use over_there_transport::Transmitter;

    fn new_transmitter<'a>(
        transmission_size: usize,
    ) -> Transmitter<'a, NoopAuthenticator, NoopBicrypter> {
        Transmitter::new(transmission_size, &NoopAuthenticator, &NoopBicrypter)
    }

    fn new_receiver<'a>(
        transmission_size: usize,
    ) -> Receiver<'a, NoopAuthenticator, NoopBicrypter> {
        use std::time::Duration;
        let cache_capacity = 1500;
        let cache_duration = Duration::from_secs(5 * 60);
        Receiver::new(
            transmission_size,
            cache_capacity,
            cache_duration,
            &NoopAuthenticator,
            &NoopBicrypter,
        )
    }

    #[test]
    fn recv_should_fail_if_unable_to_receive_data() {
        let r = new_receiver(100);
        let m = MsgReceiver::new(&r);

        match m.recv(|_| Err(std::io::Error::from(std::io::ErrorKind::Other))) {
            Err(MsgReceiverError::RecvData(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn recv_should_fail_if_unable_to_convert_complete_data_to_msg() {
        let r = new_receiver(100);
        let m = MsgReceiver::new(&r);

        // Construct a data representation that is valid to read
        // but is not a msg
        let data: [u8; 100] = {
            let t = new_transmitter(100);
            let mut tmp = [0; 100];
            t.send(&vec![1, 2, 3], |msg_data| {
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
            Err(MsgReceiverError::DecodeMsg(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn recv_should_succeed_if_able_to_receive_msg() {
        let r = new_receiver(200);
        let m = MsgReceiver::new(&r);
        let msg = Msg::from(Content::HeartbeatRequest);

        // Construct a data representation for our message
        // NOTE: With addition of a 256-bit (32 byte) message signature,
        //       we've moved from a message of ~90 bytes to ~120 bytes,
        //       so we have to increase the data buffer beyond 100
        let data: [u8; 200] = {
            let t = new_transmitter(100);
            let mut tmp = [0; 200];
            t.send(&msg.to_vec().unwrap(), |msg_data| {
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
