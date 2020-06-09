mod tcp;
mod udp;

use crate::core::Msg;

use log::{error, trace, warn};
use crate::transport::InboundWireError;
use std::net::SocketAddr;
use tokio::{sync::mpsc, task};

pub struct EventManager {
    inbound_handle: task::JoinHandle<()>,
    outbound_handle: task::JoinHandle<()>,
    tx: mpsc::Sender<Vec<u8>>,
}

impl EventManager {
    pub async fn send(&mut self, data: Vec<u8>) -> Result<(), Vec<u8>> {
        self.tx.send(data).await.map_err(|x| x.0)
    }

    pub async fn wait(self) -> Result<(), task::JoinError> {
        tokio::try_join!(self.inbound_handle, self.outbound_handle).map(|_| ())
    }
}

pub struct AddrEventManager {
    inbound_handle: task::JoinHandle<()>,
    outbound_handle: task::JoinHandle<()>,
    tx: mpsc::Sender<(Vec<u8>, SocketAddr)>,
}

impl AddrEventManager {
    pub async fn send_to(
        &mut self,
        data: Vec<u8>,
        addr: SocketAddr,
    ) -> Result<(), (Vec<u8>, SocketAddr)> {
        self.tx.send((data, addr)).await.map_err(|x| x.0)
    }

    pub async fn wait(self) -> Result<(), task::JoinError> {
        tokio::try_join!(self.inbound_handle, self.outbound_handle).map(|_| ())
    }
}

/// Process result of receiving data, indicating whether should continue
/// processing additional data
async fn process_inbound<T>(
    result: Result<(Option<Vec<u8>>, SocketAddr), InboundWireError>,
    sender: mpsc::Sender<T>,
    mut on_inbound_tx: mpsc::Sender<(Msg, SocketAddr, mpsc::Sender<T>)>,
) -> bool
where
    T: Send + 'static,
{
    match result {
        Ok((None, _)) => true,
        Ok((Some(data), addr)) => {
            trace!("Incoming data of size {} from {}", data.len(), addr);
            match Msg::from_slice(&data) {
                Ok(msg) => {
                    trace!("Valid msg {:?} from {}", msg, addr);

                    if let Err(x) =
                        on_inbound_tx.send((msg, addr, sender)).await
                    {
                        error!("Encountered error: {}", x);
                    }

                    true
                }
                Err(x) => {
                    warn!(
                        "Discarding data of size {} as not valid msg: {}",
                        data.len(),
                        x
                    );
                    true
                }
            }
        }
        Err(x) => match x {
            InboundWireError::IO(x) => {
                error!("Fatal IO on wire: {}", x);
                false
            }
            InboundWireError::InputProcessor(x) => {
                error!("Process error on wire: {}", x);
                true
            }
        },
    }
}
