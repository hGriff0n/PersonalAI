
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::{Arc, Mutex};

use get_if_addrs;
use multimap::MultiMap;     // This is unused at the moment
use serde_json;
use tokio::io;

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
    connections: Arc<Mutex<HashMap<SocketAddr, Box<AppConnection>>>>,
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
        let mut connections = HashMap::new();
        let manager_proxy = AppConnection::manager(cancel.clone());
        connections.insert(*manager_proxy.get_addr(), Box::new(manager_proxy));

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
    fn handshake(&mut self, msg: &message::Message, addr: &SocketAddr) -> CallbackResult {
        trace!("Received handshake request from {:?}", addr);

        // Get the connection object to add the handshake data to it
        let mut conn_lock = self.connections.lock().unwrap();
        let conn = conn_lock.get_mut(&addr).expect("Attempt to perform handshake registration with unregistered connection (not in the manager connection map)");

        if let Some(uuid) = msg.sender.uuid.clone() {
            info!("Registering app uuid {:?} connected to socket address {:?}", uuid, addr);
            conn.uuid = uuid;
        }
        if let Some(role) = msg.sender.role.clone() {
            info!("Registering app role {:?} connected to socket address {:?}", role, addr);
            conn.role = role;
        }

        if let Some(ref args) = msg.args {
            if let serde_json::Value::Array(ref handles) = args[0]["registered_handles"] {
                conn.exported_handles.extend(handles.iter().filter(|handle| handle.is_string()).map(|handle| handle.as_str().unwrap().to_string()));
                info!("Registering action handles for socket address {:?}: {:?}", addr, conn.exported_handles);
            }
        }

        Ok(json!(null))
    }

    fn handle_search(&mut self, msg: &message::Message, addr: &SocketAddr) -> CallbackResult {
        trace!("Received search request from {:?}", addr);

        // Perform a filesystem search over the given arguments
        if let Some(ref args) = msg.args {
            info!("Searching for {:?}", args);

            if let Some(query) = &args[0].as_str() {
                let results = seshat::default_search(query, &self.index);
                info!("Successfully completed request for {:?}! Returning results: {:?}", query, results);
                Ok(json!(results))

            } else {
                debug!("Could not cast query arg to string: {:?}", args[0]);
                Err(DeviceErrors::Sendable("Search expects arg[0] having type `String`".to_string()))
            }
        } else {
            Err(DeviceErrors::Sendable("No arguments provided to manager search query".to_string()))
        }
    }

    fn handle_stop(&mut self, _msg: &message::Message, addr: &SocketAddr) -> CallbackResult {
        trace!("Received stop request from {:?}", addr);
        <Self as networking::BasicServer>::drop_connection(self, *addr);
        Ok(json!(null))
    }

    fn handle_quit(&mut self, _msg: &message::Message, addr: &SocketAddr) -> CallbackResult {
        trace!("Received quit request from {:?}", addr);

        // Send a close signal to all connected devices
        // NOTE: We don't remove the connections as the manager is closing anyways
        // TODO: Wouldn't this message actually be received as a broadcast?
        // TODO: Shouldn't we close the connection that gave us the message first (to prevent loops)
        let mut conns = self.connections.lock().unwrap();
        for (addr, _) in conns.iter() {
            trace!("Closing connection on {:?} in response to `quit` request", *addr);
            self.on_connection_close(&conns, *addr)?;
        }
        conns.clear();

        // Send the server close signal
        info!("Sent asynchronous close requests to all connections. Closing device manager");
        self.cancel.send(())
            .map_err(|_err| DeviceErrors::Internal("Failed to send cancel signal to the device manager".to_string()))?;

        Ok(json!(null))
    }

    fn on_connection_close(&self, conns: &HashMap<SocketAddr, Box<AppConnection>>, addr: SocketAddr) -> Result<(), DeviceErrors>{
        if let Some(ref conn) = conns.get(&addr) {
            if !conn.is_manager() {
                // TODO: Convert this to an internal error
                conn.get_closer().send(())
                    .map_err(|_err| DeviceErrors::Internal(format!("Failed to send closing signal to connection on socket address {:?}", addr)))?
            }
        }

        Ok(())
    }

    fn resolve_destination_targets(&self, dest: &message::MessageDest) -> Result<Vec<(bool, SocketAddr)>, DeviceErrors> {
        // TODO: We still have some work integrating the manager into the same `AppConnection` type
        let connections = self.connections.lock().unwrap();
        let mut conns: Box<Iterator<Item=&Box<AppConnection>>> = Box::new(connections.values());

        if !dest.broadcast.unwrap_or(false) {
            if let Some(role) = dest.role.clone() {
                debug!("Filtering connections by role: {}", role);
                conns = Box::new(conns.filter(move |conn| conn.get_role() == role));
            }

            // TODO: These types don't match yet
            // if let Some(addr) = dest.addr.clone() {
            //     debug!("Filtering connections by address: {}", addr);
            //     conns = Box::new(conns.filter(move |conn| conn.get_addr() == addr));
            // }

            if let Some(uuid) = dest.uuid.clone() {
                debug!("Filtering connections by uuid: {}", uuid);
                conns = Box::new(conns.filter(move |conn| conn.get_uuid() == uuid));
            }
        }

        let conns: Vec<(bool, SocketAddr)> = conns
            .map(|conn| (conn.is_manager(), conn.get_addr().clone()))
            .collect();
        if conns.len() == 0 {
            let error_msg = format!("No connections found for specified desination target {:?}", dest);
            error!("{}", error_msg);
            Err(DeviceErrors::Sendable(error_msg))

        } else {
            Ok(conns)
        }
    }

    fn return_to_sender(&self, mut msg: message::Message, addr: &SocketAddr) -> Result<(), DeviceErrors> {
        msg.dest = msg.sender.clone().into();
        if let Some(ref conn) = self.connections.lock().unwrap().get(&addr) {
            if let Some(ref queue) = conn.get_queue() {
                let msg = serde_json::to_value(msg)
                    .map_err(|err| DeviceErrors::Sendable(err.to_string()))?;
                queue.unbounded_send(msg)
                    .map_err(|_err| DeviceErrors::Recoverable(
                        io::Error::new(io::ErrorKind::Other, format!("Failed to send message to communication queue of connection on socket address {:?}", addr))))?;
            }

        // NOTE: We use this check to ignore messages that get sent to an app after the app "de-registers" itself
        } else if msg.action != Some("stop".to_string()) {
            debug!("Failed to send response to unrecognized address {:?}: {:?}", addr, msg);
        }

        Ok(())
    }

    fn route_server_message(&mut self, mut msg: message::Message, addr: &SocketAddr) -> Result<(), DeviceErrors> {
        trace!("Handling server request");

        // Clone the map to satisfy the borrow checker
        let handle_map = self.registered_handles.clone();

        // Dispatch the action into the registered handles
        let handles = handle_map.lock().unwrap();
        match msg.action.clone()
            .and_then(|action| handles.get(&action))
            .and_then(|handle| Some(handle(self, &msg, addr)))
            .unwrap_or(Err(DeviceErrors::Sendable("Attempt to query server using unknown action handle".to_string())))?
        {
            serde_json::Value::Null => (),
            resp => msg.resp = Some(resp)
        }

        // Return the message to the sender
        self.return_to_sender(msg, addr)
    }

    fn route_broadcast_message(&mut self, msg: message::Message, conn_opts: &Vec<(bool, SocketAddr)>) -> Result<(), DeviceErrors> {
        let msg = serde_json::to_value(msg)
            .map_err(|err| DeviceErrors::Sendable(err.to_string()))?;

        debug!("Performing broadcast of {:?} to all registered modalities", msg);

        // NOTE: We need to query the stored connections map to guard against the connection
        // dropping in between when the `conn_opts` vector was generated and this method is called
        // NOTE: We don't just use the `conns` map as we may use this system to broadcast "events"
        // Which should only be sent to apps which explicitly register themselves for them
        let conns = self.connections.lock().unwrap();
        for (_, conn_addr) in conn_opts {
            if let Some(ref conn) = conns.get(conn_addr) {
                if let Some(ref queue) = conn.get_queue() {
                    queue.unbounded_send(msg.clone())
                        .map_err(|_err| DeviceErrors::Recoverable(
                            io::Error::new(io::ErrorKind::Other, format!("Failed to send message to communication queue for connection on {:?}", conn.get_addr()))))?;
                }
            }
        }

        Ok(())
    }

    fn resolve_sender<'a>(&self, sender: &message::MessageSender, conns: &'a HashMap<SocketAddr, Box<AppConnection>>) -> Option<&'a Box<AppConnection>> {
        let sender_uuid = sender.uuid.clone();
        let sender_uuid = sender_uuid.as_ref().map(|s| &**s);       // This is solely to type the coparison
        conns.values()
            .find(|conn| Some(conn.get_uuid()) == sender_uuid)
    }

    fn route_network_message(&mut self, msg: message::Message, conn_opts: &Vec<(bool, SocketAddr)>) -> Result<(), DeviceErrors> {
        trace!("Sending the message to another modality");

        if msg.dest.broadcast.unwrap_or(false) {
            return self.route_broadcast_message(msg, conn_opts);
        }

        // TODO: Resolve the destinatino to a single connection, making sure 'handle' is valid on the app
            // TODO: We may want to "precompute" the sender app here, in order to consider it in destination selection
        // TODO: Need to store information about handles on a "per-app" basis
        let selected_connection_index = 0 as usize;
        let (_, selected_connection) = conn_opts[selected_connection_index];

        // Now that we've resolved the destination, send the messages
        let conns = self.connections.lock().unwrap();
        if let Some(ref conn) = conns.get(&selected_connection) {
            // Test that the handle is "callable" using the selected app
            // TODO: See if I could instead move this outside (to where we are selecting the app)
            match msg.action.clone() {
                Some(action) => if !conn.can_respond_to_handle(action.as_str()) {
                    error!("Could not unify role:handle to point to same app for msg {:?}", msg);
                    return Err(DeviceErrors::Sendable(format!("Action handle `{:?}` is not satisfiable under a known app with role `{:?}`", action, conn.get_role())));
                },
                None => {
                    error!("Message did not specify an action handle: {:?}", msg);
                    return Err(DeviceErrors::Sendable(format!("No action specified in received message: {:?}", msg)));
                }
            };

            // The message-app pairing is valid, send the message
            debug!("Sending message to {:?}", selected_connection);
            if let Some(ref queue) = conn.get_queue() {
                let sending_msg = serde_json::to_value(msg.clone())
                    .map_err(|err| DeviceErrors::Sendable(err.to_string()))?;
                queue.unbounded_send(sending_msg)
                    .map_err(|_err| DeviceErrors::Recoverable(
                        io::Error::new(io::ErrorKind::Other, format!("Failed to send message to communication queue for connection on {:?}", conn.get_addr()))))?;

                // Check whether we should consider producing an 'ack' message
                // NOTE: We currently leave the question of whether we should even consider producing an "ack" to the `responder`
                // This is not entireably desirable as it leaves a large hole for introducing app errors (see cli app which requires an 'ack' response)
                // TODO: Move towards a more "progressive" ack-ing mechanism where the original sending app requests that an ack "should" be sent
                if msg.send_ack {

                    // We first have to determine which connection is responsible for the sender app
                    // NOTE: We currently perform this check solely on the basis of the app's uuid
                    if let Some(sender_conn) = self.resolve_sender(&msg.sender, &conns) {

                        // If the immediate recipient cannot also send this message on to the sender, then we send the ack out to the sender connection
                        if conn.get_addr() != sender_conn.get_addr() {
                            debug!("The receiving app was not the same as the sending app. Sending ack message to {:?}", sender_conn.get_uuid());
                            if let Some(queue) = sender_conn.get_queue().clone() {
                                let mut msg = msg;
                                msg.action = Some("ack".to_string());

                                let msg = serde_json::to_value(msg)
                                    .map_err(|err| DeviceErrors::Sendable(err.to_string()))?;

                                queue.unbounded_send(msg)
                                    .map_err(|_err| DeviceErrors::Recoverable(
                                        io::Error::new(io::ErrorKind::Other, format!("Failed to send message to communication queue for connection on {:?}", conn.get_addr()))))?;
                            } else {
                                return Err(DeviceErrors::Recoverable(
                                    io::Error::new(io::ErrorKind::Other, format!("Failed to get communication queue for connection {:?}", sender_conn.get_uuid()))));
                            }
                        }
                    } else {
                        error!("Attempt to send message from unknown app {:?}: {:?}", msg.sender.uuid, msg);
                    }
                }
            } else {
                error!("Attempt to send message to unknown address {:?}: {:?}", selected_connection, msg);
            }
        }

        Ok(())
    }
}


impl networking::BasicServer for DeviceManager {
    fn handle_request(&mut self, msg: serde_json::Value, addr: &SocketAddr) -> Result<(), io::Error> {
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
        let mut original_msg = msg.clone();
        self.resolve_destination_targets(&msg.dest)
            .and_then(|conns| match conns.iter().find(|(is_manager, _)| *is_manager) {
                Some(_) => self.route_server_message(msg, addr),
                None => self.route_network_message(msg, &conns),
            })
            .or_else(|err| match err {
                DeviceErrors::Sendable(error) => {
                    original_msg.action = Some("error".to_string());
                    original_msg.resp = Some(json!(vec![json!(error)]));
                    self.return_to_sender(original_msg, addr)
                        .map_err(|_err| io::Error::new(io::ErrorKind::Other, format!("Failed to send error message to connection on socket addres: {:?}", addr)))
                },
                DeviceErrors::Recoverable(error) => Err(error),
                DeviceErrors::Internal(error) => panic!(error),
            })?;

        Ok(())
    }

    // TODO: Why do we have this method?
    #[allow(unused_variables, unused_mut)]
    fn handle_response(&mut self, mut msg: serde_json::Value, addr: &SocketAddr) -> serde_json::Value {
        msg
    }

    fn add_connection(&self, addr: SocketAddr, close_signal: Closer, write_signal: Communicator) -> Result<(), io::Error> {
        trace!("Adding connection to {:?}", addr);

        let mut conns = self.connections.lock().unwrap();
        conns.insert(addr, Box::new(AppConnection::new(addr.clone(), close_signal, write_signal)));

        info!("Added connection to {:?}", addr);
        Ok(())
    }

    // TODO: Change the return type of this to `Result<(), Error>`
    fn drop_connection(&mut self, addr: SocketAddr) {
        trace!("Dropping connection to {:?}", addr);

        let mut conns = self.connections.lock().unwrap();
        self.on_connection_close(&conns, addr)
            .expect(&format!("Failed to successfully close connection on socket address {:?}", addr));
        conns.remove(&addr);

        info!("Dropped connection to {:?}", addr);
    }
}


// TODO: This is the first interface to attempting to unify the handling of app, manager, and forwarding connections
trait RoutingConnection: Send{
    fn get_addr(&self) -> &SocketAddr;
    fn get_closer(&self) -> &Closer;
    fn get_queue(&self) -> &Option<Communicator>;
    fn get_role(&self) -> &str;
    fn get_uuid(&self) -> &str;
    fn can_respond_to_handle(&self, handle: &str) -> bool;
    fn is_manager(&self) -> bool;
}

// Structure to encapsulate connection state for storage
// TODO: We currently also use this to "hold" the manager self connection reference
struct AppConnection {
    pub addr: SocketAddr,
    pub close: Closer,
    pub queue: Option<Communicator>,
    pub role: String,
    pub uuid: String,
    pub exported_handles: Vec<String>,
    manager_self_connection: bool,
}

impl AppConnection {
    pub fn new(addr: SocketAddr, close: Closer, queue: Communicator) -> Self {
        Self{
            addr: addr,
            close: close,
            queue: Some(queue),
            role: "".to_string(),
            uuid: "".to_string(),
            exported_handles: Vec::new(),
            manager_self_connection: false
        }
    }

    pub fn manager(closer: Closer) -> Self {
        Self{
            addr: "0.0.0.0:0".parse::<SocketAddr>().unwrap(),
            close: closer,
            queue: None,
            role: "manager".to_string(),
            uuid: "".to_string(),
            exported_handles: Vec::new(),
            manager_self_connection: true
        }
    }
}

impl RoutingConnection for AppConnection {
    fn get_addr(&self) -> &SocketAddr {
        &self.addr
    }
    fn get_closer(&self) -> &Closer {
        &self.close
    }
    fn get_queue(&self) -> &Option<Communicator> {
        &self.queue
    }
    fn get_role(&self) -> &str {
        &self.role
    }
    fn get_uuid(&self) -> &str {
        &self.uuid
    }
    fn is_manager(&self) -> bool {
        self.manager_self_connection
    }

    fn can_respond_to_handle(&self, handle: &str) -> bool {
        self.exported_handles.iter().find(|&s| s == handle).is_some()
    }
}

// Types for the device callback functions
type CallbackResult = Result<serde_json::Value, DeviceErrors>;
type DeviceCallback = fn(&mut DeviceManager, &message::Message, addr: &SocketAddr) -> CallbackResult;

#[derive(Debug)]
enum DeviceErrors {
    Recoverable(io::Error),
    Sendable(String),
    Internal(String)
}
