
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::{Arc, Mutex};

use get_if_addrs;
use multimap::MultiMap;     // This is unused at the moment
use serde_json;
use tokio::io::{Error, ErrorKind};

use networking;
use networking::{Closer, Communicator};

use seshat;
use seshat::index as idx;

use message;


// TODO: We need to modify the `addr` portion of destination resolution due to the type system
    // Apps will probably require it to be an `IpAddr`
    // We can fill it in by checking the route field (if it's empty, I'm the first to see it)
#[derive(Clone)]
pub struct DeviceManager {
    connections: Arc<Mutex<HashMap<SocketAddr, DestinationTarget>>>,
    registered_handles: Arc<Mutex<HashMap<String, DeviceCallback>>>,

    cancel: Closer,
    index: idx::Index,
    public_ip: IpAddr,
}

impl DeviceManager {
    pub fn get_index(&self) -> &idx::Index {
        &self.index
    }

    pub fn new(index: idx::Index, cancel: Closer) -> Self {
        // Extract the device's public ip (NOTE: For now I'm just taking the first non-localhost interface on the system)
        let my_public_ip = get_if_addrs::get_if_addrs()
            .ok()
            .and_then(|ifaces| ifaces.iter()
                .map(|iface| iface.ip().clone())
                .filter(|&addr| match addr {
                    IpAddr::V4(addr) => addr != Ipv4Addr::LOCALHOST,
                    IpAddr::V6(addr) => addr != Ipv6Addr::LOCALHOST,
                })
                .next())
            .expect("Failed to determine local public ip address");
        info!("Calculated public ip address: {:?}", my_public_ip);

        // Register the server message callbacks
        let mut handle_map = HashMap::<String, DeviceCallback>::new();
        handle_map.insert("handshake".to_string(), Self::handshake);
        handle_map.insert("search".to_string(), Self::handle_search);
        handle_map.insert("stop".to_string(), Self::handle_stop);
        handle_map.insert("quit".to_string(), Self::handle_quit);

        // Add the manager into the "connections" list
        let connections = HashMap::new();
        connections.insert(manager_proxy_addr, Connection::manager(cancel.clone()));

        // Finalize the device manager
        Self{
            connections: Arc::new(Mutex::new(connections)),
            registered_handles: Arc::new(Mutex::new(handle_map)),
            cancel: cancel,
            index: index,
            public_ip: my_public_ip
        }
    }

    // TODO: How do we handle "multi-stage" handshakes with this format?
    fn handshake(&mut self, msg: &mut message::Message, addr: &SocketAddr) -> CallbackResult {
        trace!("Received handshake request from {:?}", addr);

        // Get the connection object to modify it
        let mut conn_lock = self.connections.lock().unwrap();
        let mut conn = conn_lock.get_mut(&addr);
        if conn.is_none() {
            return Err(Error::new(ErrorKind::NotConnected, "No connection found for the specified address"));
        }

        // Add the handshake data to the connection object
        let mut conn = conn.unwrap();
        if let Some(uuid) = msg.sender.uuid.clone() {
            info!("Registering app uuid {:?} connected to socket address {:?}", uuid, addr);
            conn.uuid = uuid;
        }
        if let Some(role) = msg.sender.uuid.clone() {
            info!("Registering app role {:?} connected to socket address {:?}", role, addr);
            conn.role = role;
        }

        None
    }

    fn handle_search(&mut self, msg: &mut message::Message, addr: &SocketAddr) -> CallbackResult {
        trace!("Received search request from {:?}", addr);

        // Perform a filesystem search over the given arguments
        if let Some(ref args) = msg.args {
            info!("Searching for {:?}", args);

            if let Some(query) = &args[0].as_str() {
                let results = seshat::default_search(query, &self.index);
                msg.resp = Some(json!(results));
                info!("Found results: {:?}", msg.resp);

            } else {
                debug!("Could not cast query arg to string: {:?}", args[0]);
            }
        }

        None
    }

    fn handle_stop(&mut self, msg: &mut message::Message, addr: &SocketAddr) -> CallbackResult {
        trace!("Received stop request from {:?}", addr);
        <Self as networking::BasicServer>::drop_connection(self, *addr);
        None
    }

    fn handle_quit(&mut self, msg: &mut message::Message, addr: &SocketAddr) -> CallbackResult {
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
        Some(self.cancel.send(())
            .map_err(|_| Error::new(ErrorKind::ConnectionAborted, "Failed to send cancel signal")))
    }

    fn on_connection_close(&self, conns: &HashMap<SocketAddr, Connection>, addr: SocketAddr) {
        if let Some(ref conn) = conns.get(&addr) {
            if !conn.is_manager {
                conn.close.send(())
                    .expect("Failed to send closing signal to connection");
            }
        }
    }

    fn resolve_destination_target(&self, dest: &message::MessageDest) -> Result<Vec<Connection>, Error> {
        // TODO: We still have some work integrating the manager into the same `Connection` type
        let mut conns = self.connections.lock().unwrap().values().iter();

        if let Some(role) = dest.role {
            debug!("Filtering connections by role: {}", role);
            conns = conns.filter(|conn| conn.role == role);
        }

        // TODO: These types don't match yet
        // if let Some(addr) = dest.addr {
        //     debug!("Filtering connections by address: {}", addr);
        //     conns = conns.filter(|conn| conn.addr == addr);
        // }

        if let Some(uuid) = dest.uuid {
            debug!("Filtering connections by uuid: {}", addr);
            conns = conns.filter(|conn| conn.uuid == uuid);
        }

        let conns: Vec<Connection> = conns.collect();
        if conns.len() == 0 {
            error!("No connections found for specified destination target {:?}", dest);
            Err(Error::new(ErrorKind::InvalidInput, "No connections regsitered for destination target"));

        } else {
            Ok(conns)
        }
    }

    fn route_server_message(&mut self, mut msg: message::Message, addr: &SocketAddr) -> Result<(), Error> {
        trace!("Handling server request");

        // Clone the map to satisfy the borrow checker
        let handle_map = self.registered_handles.clone();

        // Dispatch the action into the registered handles
        let action = msg.action.clone().unwrap_or(UNMATCHABLE_STRING.to_string());
        let handles = handle_map.lock().unwrap();
        if let Some(result) = handles.get(&action)
            .and_then(|handle| handle(self, &mut msg, addr))
        {
            return result;
        }

        // Return the message to the sender
        msg.dest = msg.sender.clone().into();
        if let Some(ref conn) = self.connections.lock().unwrap().get(&addr) {
            if let Some(queue) = conn.queue {
                queue.unbounded_send(serde_json::to_value(msg)?)
                    .map_err(|_err|
                        Error::new(ErrorKind::Other, "Failed to send message through pipe"))?;
            }

        } else if action != "stop" {
            debug!("Failed to send response to unrecognized address {:?}: {:?}", addr, msg);
        }

        Ok(())
    }

    fn route_network_message(&mut self, msg: message::Message, conn_opts: &Vec<Connection>) -> Result<(), Error> {
        trace!("Sending the message to another modality");

        Ok(())
    }
}


impl networking::BasicServer for DeviceManager {
    fn handle_request(&mut self, msg: serde_json::Value, addr: &SocketAddr) -> Result<(), Error> {
        let mut msg: message::Message = serde_json::from_value(msg)?;
        debug!("Parsed received message {:?}", msg);

        // 1) Append the current device addr to the route array
        // 2) Set the sender's addr value if not already set
        msg.route.push(self.public_ip);
        if msg.sender.addr.is_none() {
            msg.sender.addr = Some(self.public_ip);
        }
        trace!("Appended required sender data");

        // Handle the message as requested by the sender
        let conns = self.resolve_destination_targets(&msg.dest)?;
        match conns.iter().find(|conn| conn.is_manager) {
            Some(_) => self.route_server_message(msg, addr)?,
            None => self.route_network_message(msg, conns)?
        }

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
        conns.insert(addr, Connection::new(addr.clone(), close_signal, write_signal));\

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


enum DestinationTarget {
    Manager,
    App(Vec<Connection>)
}

// Structure to encapsulate connection state for storage
struct Connection {
    pub addr: SocketAddr,
    pub close: Closer,
    pub queue: Option<Communicator>,
    pub role: String,
    pub uuid: String
    pub is_manager: bool
}

impl Connection {
    pub fn new(addr: SocketAddr, close: Closer, queue: Communicator) -> Self {
        Self{
            addr: addr,
            close: close,
            queue: Some(queue),
            role: "".to_string(),
            uuid: "".to_string(),
            is_manager: false
        }
    }

    pub fn manager(closer: Closer) -> Self {
        Self{
            addr: "0.0.0.0:0".parse::<SocketAddr>().unwrap(),
            close: closer,
            queue: None,
            role: "manager".to_string(),
            uuid: "".to_string(),
            is_manager: true
        }
    }
}

// Types for the device callback functions
type CallbackResult = Option<Result<(), Error>>;        // Some(_) indicates return early with the result
type DeviceCallback = fn(&mut DeviceManager, &mut message::Message, addr: &SocketAddr) -> CallbackResult;

// NOTE: This is used to get around the borrow checker when matching against the `message` structs
// For some reason, the borrow checker wouldn't allow me to transform an `Option<String>` into an `Option<&str>` temporarily
const UNMATCHABLE_STRING: &'static str = "DO_NOT_MATCH_THIS_STRING";
