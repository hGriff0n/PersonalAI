
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::{Arc, Mutex};

use get_if_addrs;
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
    pub role: String,
    pub uuid: String
}

impl Connection {
    pub fn new(addr: SocketAddr, close: Closer, queue: Communicator) -> Self {
        Self{
            addr: addr,
            close: close,
            queue: queue,
            role: "".to_string(),
            uuid: "".to_string(),
        }
    }
}

#[derive(Clone)]
pub struct DeviceManager {
    connections: Arc<Mutex<HashMap<SocketAddr, Connection>>>,
    role_map: Arc<Mutex<MultiMap<String, SocketAddr>>>,
    uuid_map: Arc<Mutex<HashMap<String, SocketAddr>>>,

    cancel: Closer,
    index: idx::Index,

    // NOTE: We can remove the option once we can determine the device's public ip addr
    public_ip: IpAddr
}

// TODO: I need to add in the capability to recognize sent messages (for broadcasts specifically)
// TODO: I want to have the device's address here
impl DeviceManager {
    pub fn new(index: idx::Index, cancel: Closer) -> Self {
        // Extract the device's public ip (NOTE: For now I'm just taking the first non-localhost interface on the system)
        let my_public_ip = get_if_addrs::get_if_addrs()
            .unwrap()
            .iter()
            .map(|iface| iface.ip().clone())
            .filter(|&addr| match addr {
                IpAddr::V4(addr) => addr != Ipv4Addr::LOCALHOST,
                IpAddr::V6(addr) => addr != Ipv6Addr::LOCALHOST,
            })
            .next()
            .unwrap();
        info!("Calculated public ip address: {:?}", my_public_ip);

        Self{
            connections: Arc::new(Mutex::new(HashMap::new())),
            role_map: Arc::new(Mutex::new(MultiMap::new())),
            uuid_map: Arc::new(Mutex::new(HashMap::new())),
            cancel: cancel,
            index: index,
            public_ip: my_public_ip,
        }
    }

    pub fn get_index(&self) -> &idx::Index {
        &self.index
    }

    // TODO: Correctly implement this to clean out the whole cache
    fn on_connection_close(&self, conns: &HashMap<SocketAddr, Connection>, addr: SocketAddr) {
        let mut role_map = self.role_map.lock().unwrap();

        if let Some(ref conn) = conns.get(&addr) {
            // for role in &conn.roles {
            //     let vec = role_map.get_vec_mut(role).unwrap();
            //     vec.iter()
            //         .position(|ad| *ad == addr)
            //         .map(|e| vec.remove(e));
            // }

            conn.close.send(()).expect("Failed to send closing signal to communicator");

        } else {
            debug!("Couldn't close connection {:?}: Connection was not found in the connections map", addr);
        }

        // NOTE: We purposefully do not remove the connection from the connection map here
        // TODO: This is an optimization for "quit", could we also get this optimization for the 'role_map'
    }

    // Resolve who sent the message
    fn resolve_connection(&self, _send: &message::MessageSender) -> Option<Option<SocketAddr>> {
        Some(None)
    }

    // Resolve where the message is being requested to be directed
    fn resolve_destination(&self, dest: &message::MessageDest) -> Option<Option<SocketAddr>> {
        trace!("Resolving destination labels to sending socket address");

        // If the specific app is specified, send it there
        if let Some(ref uuid) = dest.uuid {
            let uuid_map = self.uuid_map.lock().unwrap();
            if uuid_map.contains_key(uuid) {
                return Some(uuid_map.get(uuid).map(|addr| addr.to_owned()));
            }

            debug!("Requested sending to uuid {:?} but no such application was found", uuid);
        }

        // If the device IP is specified, send it there
        // NOTE: This won't currently work, because we don't send things correctly
        // if let Some(addr) = dest.addr {
        //     if self.connections.lock().unwrap().contains_key(&addr) {
        //         return addr.clone();
        //     }
        // }

        let role = dest.role.clone().unwrap_or(UNMATCHABLE_STRING.to_string());
        let dest = match role.as_str() {
            "manager" => None,
            "device" => None,
            role => Some(self.role_map.lock().unwrap().get(role).map(|addr| addr.clone()))
        };

        // Log resolution status
        match dest {
            Some(Some(addr)) => debug!("Resolved destination connection: {:?}", addr),
            Some(None) => debug!("Failed to resolve destination: No connection registered for {:?}", role),
            None => debug!("Resolved destination connection: device-manager"),
        }

        dest
    }

    // Handle any server specific requests
    fn handle_message(&mut self, mut msg: message::Message, addr: &SocketAddr) -> Result<(), Error> {
        trace!("Handling server request");

        let action = msg.action.clone().unwrap_or(UNMATCHABLE_STRING.to_string());
        match action.as_str() {
            "handshake" => {
                trace!("Received handshake request from {:?}", addr);

                // NOTE: This may not borrow check
                let mut conn_lock = self.connections.lock().unwrap();
                let mut conn = conn_lock.get_mut(&addr);

                if let Some(uuid) = msg.sender.uuid.clone() {
                    info!("Adding uuid {:?} to point to socket address {:?}", uuid, addr);
                    self.uuid_map.lock().unwrap().insert(uuid.clone(), addr.clone());
                    if let Some(ref mut conn) = conn {
                        conn.uuid = uuid;
                    }
                }

                if let Some(role) = msg.sender.role.clone() {
                    info!("Adding role {:?} to point to socket address {:?}", role, addr);
                    self.role_map.lock().unwrap().insert(role.clone(), addr.clone());
                    if let Some(ref mut conn) = conn {
                        conn.role = role;
                    }
                }
            },
            "search" => {
                trace!("Received search request from {:?}", addr);

                // Perform a filesystem search over the given arguments
                if let Some(ref args) = msg.args {
                    info!("Searching for {:?}", args);

                    let query = &args[0].as_str().unwrap();
                    let results = seshat::default_search(query, &self.index);
                    msg.resp = Some(json!(results));

                    info!("Found results: {:?}", msg.resp);
                }
            },
            "stop" => {
                trace!("Received stop request from {:?}", addr);
                <Self as networking::BasicServer>::drop_connection(self, *addr)
            },
            "quit" => {
                trace!("Received quit request from {:?}", addr);

                // Send a close signal to all connected devices
                // NOTE: We don't remove the connections as the manager is closing anyways
                // TODO: Wouldn't this message actually be received as a broadcast?
                // TODO: Shouldn't we close the connection that gave us the message first (to prevent loops)
                let mut conns = self.connections.lock().unwrap();
                for (addr, _) in conns.iter() {
                    trace!("Closing connection on {:?} in response to `quit` request", *addr);
                    self.on_connection_close(&conns, *addr);
                }
                conns.clear();
                info!("Sent asynchronous close requests to all connections. Closing device manager");

                // Send the server close signal
                return self.cancel.send(())
                    .map_err(|_| Error::new(ErrorKind::ConnectionAborted, "Failed to send cancel signal"));
            },
            _ => ()
        };

        // Return the message to the sender
        msg.dest = msg.sender.clone().into();
        if let Some(ref conn) = self.connections.lock().unwrap().get(&addr) {
            conn.queue.unbounded_send(serde_json::to_value(msg).unwrap());

        } else if action != "stop" {
            debug!("Failed to send response to unrecognized address {:?}: {:?}", addr, msg);
        }
        Ok(())
    }

    // Handle routing the message to the requested destination
    fn route_message(&mut self, msg: message::Message, dest: Option<SocketAddr>) -> Result<(), Error> {
        trace!("Sending the message to another modality");

        if !msg.dest.broadcast.unwrap_or(false) {
            // Produce a list of the connection sinks that we want to send the message to
            // NOTE: This allows us to turn the 'dest' field into an array
            let mut send_queue = Vec::new();

            debug!("Routing the message according to it's `dest` field");

            // Add the specified destination device to the queue
            if let Some(dest) = dest {
                if let Some(ref conn) = self.connections.lock().unwrap().get(&dest) {
                    send_queue.push((conn.queue.clone(), false));
                    debug!("Adding {:?} to the send queue for message reception", dest);

                    // Send an ack message to the original sender if desired
                    if let Some(Some(sender)) = self.resolve_connection(&msg.sender) {
                        if sender != dest {
                            debug!("The receiving app was not the same as the sending message. Adding ack message to {:?} to sending queue", sender);

                            if let Some(ref conn) = self.connections.lock().unwrap().get(&sender) {
                                send_queue.push((conn.queue.clone(), true));
                            } else {
                                debug!("Failed to send ack message to unknown addres {:?}: {:?}", sender, msg);
                            }
                        }
                    }

                } else {
                    debug!("Failed to send message to unknown address {:?}: {:?}", dest, msg);
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

            debug!("Performing broadcast of {:?} to all registered modalities", msg);
            for (_, ref conn) in self.connections.lock().unwrap().iter() {
                conn.queue.unbounded_send(msg.clone());
            }
        }

        Ok(())
    }
}

impl networking::BasicServer for DeviceManager {
    fn handle_request(&mut self, msg: serde_json::Value, addr: &SocketAddr) -> Result<(), Error> {
        let mut msg: message::Message = serde_json::from_value(msg)?;
        debug!("Parsed message {:?}", msg);

        // 1) Append the current device addr to the route array
        // 2) Set the sender's addr value if not already set
        msg.route.push(self.public_ip);
        if msg.sender.addr.is_none() {
            msg.sender.addr = Some(self.public_ip);
        }
        trace!("Appended required sender data");

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
        trace!("Adding connection to {:?}", addr);
        let mut conns = self.connections.lock().unwrap();
        conns.insert(addr, Connection::new(addr.clone(), close_signal, write_signal));
        info!("Added connection to {:?}", addr);
        Ok(())
    }

    // TODO: Change the return type of this to `Result<(), Error>`
    fn drop_connection(&mut self, addr: SocketAddr) {
        trace!("Dropping connection to {:?}", addr);
        let mut conns = self.connections.lock().unwrap();
        self.on_connection_close(&conns, addr);
        conns.remove(&addr);
        info!("Dropped connection to {:?}", addr);
    }
}

// NOTE: This is used to get around the borrow checker when matching against the `message` structs
// For some reason, the borrow checker wouldn't allow me to transform an `Option<String>` into an `Option<&str>` temporarily
const UNMATCHABLE_STRING: &'static str = "DO_NOT_MATCH_THIS_STRING";
