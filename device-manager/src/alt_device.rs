
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use multimap::MultiMap;
use serde_json;
use tokio::io::{Error, ErrorKind};

use networking;
use networking::{Closer, Communicator};

use seshat;
use seshat::index as idx;

use message;

struct Connection {
    pub addr: SocketAddr,
    pub close: Closer,
    pub queue: Communicator,
    pub roles: Vec<String>,
}

impl Connection {
    pub fn new(addr: SocketAddr, close: Closer, queue: Communicator) -> Self {
        Self{
            addr: addr,
            close: close,
            queue: queue,
            roles: Vec::new()
        }
    }
}

#[derive(Clone)]
pub struct DeviceManager {
    connections: Arc<Mutex<HashMap<SocketAddr, Connection>>>,
    role_map: Arc<Mutex<MultiMap<String, SocketAddr>>>,

    cancel: Closer,
    index: idx::Index,

    // NOTE: We can remove the option once we can determine the device's public ip addr
    device_addr: Option<SocketAddr>
}

// NOTE: This should give us the public ip. Not sure how well it'd works
// fn resolve_ip(host: &str) -> io::Result<Vec<IpAddr>> {
//     (host, 0).to_socket_addrs().map(|iter| iter.map(|sock| sock.ip()).collect())
// }

// TODO: I need to add in the capability to recognize sent messages (for broadcasts specifically)
// TODO: I want to have the device's address here
impl DeviceManager {
    pub fn new(index: idx::Index, cancel: Closer) -> Self {
        Self{
            connections: Arc::new(Mutex::new(HashMap::new())),
            role_map: Arc::new(Mutex::new(MultiMap::new())),
            cancel: cancel,
            index: index,
            device_addr: None,      // TODO: we need to get the device's ip addr (ie. where are we listening?)
        }
    }

    pub fn get_index(&self) -> &idx::Index {
        &self.index
    }

    fn on_connection_close(&self, conns: &HashMap<SocketAddr, Connection>, addr: SocketAddr) {
        let mut role_map = self.role_map.lock().unwrap();

        let conn = &conns[&addr];
        for role in &conn.roles {
            let vec = role_map.get_vec_mut(role).unwrap();
            vec.iter()
                .position(|ad| *ad == addr)
                .map(|e| vec.remove(e));
        }

        conn.close.send(()).expect("Failed to close connection");
        // NOTE: We purposefully do not remove the connection from the connection map here
        // TODO: This is an optimization for "quit", could we also get this optimization for 'role'
    }

    // Resolve who sent the message
    fn resolve_connection(&self, _send: &message::MessageSender) -> Option<Option<SocketAddr>> {
        Some(None)
    }

    // Resolve where the message is being requested to be directed
    fn resolve_destination(&self, dest: &message::MessageDest) -> Option<Option<SocketAddr>> {
        let role = dest.role.clone().unwrap_or(UNMATCHABLE_STRING.to_string());
        match role.as_str() {
            "manager" => None,
            "search" => None,
            _ => Some(None)
        }
    }

    // Handle any server specific requests
    fn handle_message(&mut self, mut msg: message::Message, addr: &SocketAddr) -> Result<(), Error> {
        let action = msg.action.clone().unwrap_or(UNMATCHABLE_STRING.to_string());
        match action.as_str() {
            "handshake" => {
                if let Some(ref roles) = msg.args {
                    let roles: Vec<String> = roles.iter().map(|key| key.as_str().unwrap().to_string()).collect();

                    // Register the role keys for the connection
                    if let Some(conn) = self.connections.lock().unwrap().get_mut(&addr) {
                        conn.roles = roles.clone();
                    }

                    // Register the connection for the role keys
                    let mut role_map = self.role_map.lock().unwrap();
                    for role in roles.into_iter() {
                        role_map.insert(role, addr.clone());
                    }
                }

                // TODO: Should this be an error?
            },
            "search" => {
                if let Some(ref args) = msg.args {
                    let query = &args[0].as_str().unwrap();
                    let results = seshat::default_search(query, &self.index);
                    msg.resp = Some(json!(results));
                }
            },
            "stop" => <Self as networking::BasicServer>::drop_connection(self, *addr),
            "quit" => {
                // Send a close signal to all connected devices
                // NOTE: We don't remove the connections as the manager is closing anyways
                // TODO: Wouldn't this message actually be received as a broadcast?
                // TODO: Shouldn't we close the connection that gave us the message first (to prevent loops)
                let mut conns = self.connections.lock().unwrap();
                for (addr, _) in conns.iter() {
                    self.on_connection_close(&conns, *addr);
                }

                // Send the server close signal
                return self.cancel.send(())
                    .map_err(|_| Error::new(ErrorKind::ConnectionAborted, "Failed to send cancel signal"));
            },
            _ => ()
        };

        // Return the message to the sender
        msg.dest = msg.sender.clone().into();
        let ref conn = self.connections.lock().unwrap()[addr];
        conn.queue.unbounded_send(serde_json::to_value(msg).unwrap());
        Ok(())
    }

    // Handle routing the message to the requested destination
    fn route_message(&mut self, msg: message::Message, dest: Option<SocketAddr>) -> Result<(), Error> {
        if !msg.dest.broadcast.unwrap_or(false) {
            // Produce a list of the connection sinks that we want to send the message to
            // NOTE: This allows us to turn the 'dest' field into an array
            let mut send_queue = Vec::new();

            // Add the specified destination device to the queue
            if let Some(dest) = dest {
                let ref conn = self.connections.lock().unwrap()[&dest];
                send_queue.push((conn.queue.clone(), false));
                debug!("Sending message to {:?}", dest);

                // Send an ack message to the original sender if desired
                if let Some(Some(sender)) = self.resolve_connection(&msg.sender) {
                    if sender != dest {
                        let ref conn = self.connections.lock().unwrap()[&sender];
                        send_queue.push((conn.queue.clone(), true));
                        debug!("Sending ack to {:?}", sender);
                    }
                }
            }

            // Send the json message to every connection in the queue
            for (sink, is_ack) in &send_queue {
                let mut msg = msg.clone();
                if *is_ack {
                    msg.action = Some("ack".to_string());
                }
                sink.unbounded_send(serde_json::to_value(msg).unwrap());
            }

        // Otherwise send a broadcast message to all connections
        } else {
            let msg = serde_json::to_value(msg).unwrap();

            for (_, ref conn) in self.connections.lock().unwrap().iter() {
                debug!("Broadcasting message to {:?}", conn.addr);
                conn.queue.unbounded_send(msg.clone());
            }
        }

        Ok(())
    }
}

impl networking::BasicServer for DeviceManager {
    fn handle_request(&mut self, msg: serde_json::Value, addr: &SocketAddr) -> Result<(), Error> {
        debug!("Got {:?} from {:?}", msg, addr);
        let mut msg: message::Message = serde_json::from_value(msg)?;
        debug!("Parsed message struct");

        // 1) Append the current device addr to the route array
        // 2) Set the sender's addr value if not already set
        msg.route.push(self.device_addr.unwrap());
        if msg.sender.addr.is_none() {
            msg.sender.addr = self.device_addr;
        }

        // Handle the message as requested by the sender
        match self.resolve_destination(&msg.dest) {
            None => self.handle_message(msg, addr)?,
            Some(dest) => self.route_message(msg, dest)?
        };

        Ok(())
    }

    // TODO: Why do we have this method?
    #[allow(unused_variables, unused_mut)]
    fn handle_response(&mut self, mut msg: serde_json::Value, addr: &SocketAddr) -> serde_json::Value {
        msg
    }

    fn add_connection(&self, addr: SocketAddr, close_signal: Closer, write_signal: Communicator) -> Result<(), Error> {
        debug!("Adding connection to {:?}", addr);
        self.connections.lock().unwrap().insert(addr.clone(), Connection::new(addr.clone(), close_signal, write_signal));
        Ok(())
    }

    // TODO: Change the return type of this to `Result<(), Error>`
    fn drop_connection(&mut self, addr: SocketAddr) {
        let mut conns = self.connections.lock().unwrap();
        self.on_connection_close(&conns, addr);
        conns.remove(&addr);
    }
}

// NOTE: This is used to get around the borrow checker when matching against the `message` structs
// For some reason, the borrow checker wouldn't allow me to transform an `Option<String>` into an `Option<&str>` temporarily
const UNMATCHABLE_STRING: &'static str = "DO_NOT_MATCH_THIS_STRING";
