
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use serde_json::Value;
use tokio::io::{Error, ErrorKind};

use networking;
use networking::{Closer, Communicator};

use seshat;
use seshat::index as idx;

#[derive(Clone)]
pub struct DeviceManager {
    conns: Arc<Mutex<HashMap<SocketAddr, (Closer, Communicator)>>>,     // addr -> (close channel, message channel)
    mapping: Arc<Mutex<HashMap<String, SocketAddr>>>,                   // role -> addr
    roles: Arc<Mutex<HashMap<SocketAddr, String>>>,                     // addr -> role
    cancel: Closer,

    index: idx::Index,                                                  // Search engine read end
}

impl DeviceManager {
    pub fn new(index: idx::Index, cancel: Closer) -> Self {
        Self{
            conns: Arc::new(Mutex::new(HashMap::new())),
            mapping: Arc::new(Mutex::new(HashMap::new())),
            roles: Arc::new(Mutex::new(HashMap::new())),
            cancel: cancel,
            index: index,
        }
    }

    fn on_connection_close(&self, conns: &HashMap<SocketAddr, (Closer, Communicator)>, addr: SocketAddr) {
        let mut roles = self.roles.lock().unwrap();
        if let Some(role) = roles.get(&addr).map(|role| role.to_owned()) {
            roles.remove(&addr);

            self.mapping.lock().unwrap().remove(&role);
            conns[&addr].0.send(()).expect("Failed to close connection");
        }
    }

    pub fn get_index(&self) -> &idx::Index {
        &self.index
    }
}

impl networking::BasicServer for DeviceManager {
    // TODO: Might want to reorganize this to maintain better & simpler tracking
    fn handle_request(&mut self, mut msg: Value, addr: &SocketAddr) -> Result<(), Error> {
        info!("Got {:?} from {:?}", msg, addr);

        // Perform server actions if requested
        // TODO: Is there anyway to set this up dynamically? (So we can register keywords outside of this context)
        match msg.get("action").and_then(|act| act.as_str()) {
            Some("handshake") => {
                let role = msg.get("hooks").unwrap()[0].as_str().unwrap();
                self.mapping.lock().unwrap().insert(role.to_string(), *addr);
                self.roles.lock().unwrap().insert(*addr, role.to_string());

                return Ok(());
            },
            Some("stop") => {
                self.drop_connection(*addr);
                return Ok(());
            },
            Some("quit") => {
                let conns = self.conns.lock().unwrap();

                for (caddr, (_close, _)) in conns.iter() {
                    self.on_connection_close(&conns, *caddr);
                }

                info!("Closing self");

                // TODO: Need to handle failure to send here
                return self.cancel.send(())
                    .map_err(|_| Error::new(ErrorKind::ConnectionAborted, "Failed to send cancel signal"));
            },
            Some("search") => {
                let query = msg.get("query")
                    .and_then(|dst| dst.as_str())
                    .and_then(|dst| Some(dst.to_string()));
                if let Some(query) = query {
                    let results = seshat::default_search(&query, &self.index);
                    msg["results"] = json!(results);

                    // TODO: Send the data to the original sender
                    // Right now this sends the information back to the 'dispatch' plugin, not the cli plugin

                } else {
                    debug!("Received search message with no query");
                }
            }
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
        let mut conns = self.conns.lock().unwrap();
        self.on_connection_close(&conns, addr);
        conns.remove(&addr);
    }
}
