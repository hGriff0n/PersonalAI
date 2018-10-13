
// use std::collections::HashMap;
use std::net::SocketAddr;
// use std::sync::{Arc, mpsc, Mutex};

use serde_json::Value;
use tokio::io::Error;

use server;
use server::{Closer, Communicator};

#[derive(Clone)]
pub struct AiManager {
    cancel: Closer
}

impl AiManager {
    pub fn new(cancel: Closer) -> Self {
        Self{
            cancel: cancel
        }
    }
}

#[allow(unused_mut, unused_variables)]
impl server::BasicServer for AiManager {
    fn handle_request(&mut self, mut msg: Value, addr: &SocketAddr) -> Result<(), Error> {
        Ok(())
    }
    fn handle_response(&mut self, msg: Value, _addr: &SocketAddr) -> Value {
        msg
    }
    fn add_connection(&self, addr: SocketAddr, close_signal: Closer, write_signal: Communicator) -> Result<(), Error> {
        Ok(())
    }
    fn drop_connection(&mut self, addr: SocketAddr) {}
}
