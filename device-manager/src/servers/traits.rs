
use std::net::SocketAddr;
use std::sync::mpsc;

use futures;
use serde_json::Value;
use tokio::io::Error;

pub type Closer = mpsc::Sender<()>;
pub type Communicator = futures::sync::mpsc::UnboundedSender<Value>;

pub trait BasicServer : Clone + Send {
    fn handle_request(&mut self, msg: Value, addr: &SocketAddr) -> Result<(), Error>;
    fn handle_response(&mut self, msg: Value, addr: &SocketAddr) -> Value;
    fn add_connection(&self, addr: SocketAddr, close_signal: Closer, write_signal: Communicator) -> Result<(), Error>;
    fn drop_connection(&self, addr: SocketAddr);
}
