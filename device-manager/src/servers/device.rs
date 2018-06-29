
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use serde_json::Value;
use tokio::io::Error;

use super::traits::*;

#[derive(Clone)]
pub struct DeviceManager {
    conns: Arc<Mutex<HashMap<SocketAddr, (Closer, Communicator)>>>,
    mapping: Arc<Mutex<HashMap<String, SocketAddr>>>,
    parent_addr: Option<SocketAddr>,
    cancel: Closer,
}

impl DeviceManager {
    pub fn new(parent_addr: Option<SocketAddr>, cancel: Closer) -> Self {
        Self{
            conns: Arc::new(Mutex::new(HashMap::new())),
            mapping: Arc::new(Mutex::new(HashMap::new())),
            parent_addr: parent_addr,
            cancel: cancel
        }
    }
}

impl BasicServer for DeviceManager {
    fn handle_request(&mut self, mut msg: Value, addr: &SocketAddr) -> Result<(), Error> {
        info!("Got {:?} from {:?}", msg, addr);

        // Perform server actions if requested
        match msg.get("action").and_then(|act| act.as_str()) {
            Some("handshake") => {
                let mut roles = self.mapping.lock().unwrap();

                let role = msg.get("hooks").unwrap()[0].as_str().unwrap();
                roles.insert(role.to_string(), *addr);

                return Ok(());
            },
            Some("stop") => {
                let mut conns = self.conns.lock().unwrap();
                conns[&addr].0.send(()).expect("Failed to close connection");
                return Ok(());
            },
            None => {
                return Ok(());
            },
            _ => ()
        }

        // Start crafting the message for fowarding
        let sender_addr = msg["from"].as_str().and_then(|addr| addr.parse::<SocketAddr>().ok());
        msg["from"] = json!(*addr);

        let dest_opt = msg.get("routing")
            .and_then(|dst| dst.as_str())
            .and_then(|dst| Some(dst.to_string()));
        if let Some(dest) = dest_opt {

            // Route the message based on the requested role
            if let Some(dest) = self.mapping.lock().unwrap().get(&dest) {
                // If the message is sent to start an app process, send an ACK to the requesting app
                    // Iff the requesting app is not the app responsible for responding
                if let Some(sender) = sender_addr {
                    if *dest != sender {
                        let mut ack = json!({ "from": sender, "routing": "sender", "action": "ack", "text": msg["text"].clone() });

                        let (_, ref sink) = self.conns.lock().unwrap()[&sender];

                        info!("Acking message to {:?}", sender);
                        sink.clone()
                            .unbounded_send(ack)
                            .expect("Failed to send ack");
                    }
                }

                let (_, ref sink) = self.conns.lock().unwrap()[dest];

                info!("Sending {:?} to {:?}", msg, dest);
                sink.clone()
                    .unbounded_send(msg)
                    .expect("Failed to send")

            // Forward a message back to the sender
            } else if dest == "sender" {
                let (_, ref sink) = self.conns.lock().unwrap()[addr];

                info!("Responding {:?} to {:?}", msg, addr);
                sink.clone()
                    .unbounded_send(msg.clone())
                    .expect("Failed to send");

            // Broadcast a message to all connected apps
            } else if dest == "broadcast" {
                let conns = self.conns.lock().unwrap();
                let iter = conns.iter();

                for (&dest, (_, sink)) in iter {
                    if dest != *addr {
                        info!("Broadcasting {:?} to {:?}", msg, dest);
                        sink.clone()
                            .unbounded_send(msg.clone())
                            .expect("Failed to send broadcast");
                    }
                }
            }
        }

        Ok(())
    }

    #[allow(unused_variables, unused_mut)]
    fn handle_response(&mut self, mut msg: Value, addr: &SocketAddr) -> Value {
        // msg["resp"] = json!("World");

        // if !msg.get("was_handshake").unwrap().as_bool().unwrap() {
        //     msg["play"] = json!("Aerosmith");
        // }

        msg
    }

    fn add_connection(&self, addr: SocketAddr, close_signal: Closer, write_signal: Communicator) -> Result<(), Error> {
        self.conns.lock().unwrap().insert(addr, (close_signal, write_signal));
        Ok(())
    }

    fn drop_connection(&self, addr: SocketAddr) {
        self.conns.lock().unwrap().remove(&addr);
    }
}
