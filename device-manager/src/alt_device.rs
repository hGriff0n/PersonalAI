
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use serde_json;
use tokio::io::{Error, ErrorKind};

use networking;
use networking::{Closer, Communicator};

use seshat;
use seshat::index as idx;

use msg as message;

#[derive(Clone)]
pub struct DeviceManager {
    connections: Arc<Mutex<HashMap<SocketAddr, (Closer, Communicator)>>>,

    cancel: Closer,
    index: idx::Index,

    // TODO: Do we need the `Option` to delay the initialization
    device_addr: Option<SocketAddr>
}

// TODO: I need to add in the capability to recognize sent messages (for broadcasts specifically)
// TODO: I want to have the device's address here
impl DeviceManager {
    pub fn new(index: idx::Index, cancel: Closer) -> Self {
        Self{
            connections: Arc::new(Mutex::new(HashMap::new())),
            cancel: cancel,
            index: index,
            device_addr: None,      // TODO: we need to get the device's ip addr (ie. where are we listening?)
        }
    }

    pub fn set_addr(&mut self, addr: SocketAddr) {
        self.device_addr = Some(addr);
    }

    pub fn get_index(&self) -> &idx::Index {
        &self.index
    }

    // Resolve where the message is being requested to be directed
    type ResolvedDest = Option<SocketAddr>;
    fn resolve_connection<T: message::Locateable>(&self, dest: T) -> Option<ResolvedDest> {
        let (_uuid, _addr, _role) = dest.location();

        Ok(None)
    }

    fn resolve_destination(&self, dest: message::MessageDest) -> Option<ResolvedDest> {
        let (_uuid, _addr, role) = dest.location();

        match role {
            Some("manager") => return None,
            _ => ()
        };

        Ok(None)
    }

    fn handle_message(&mut self, msg: message::Message) -> Result<(), Error> {
        match msg.action {
            Some("handshake") => {
                // TODO: Register the new connection under the specific roles
                if let Some(roles) = msg.args {

                }
                // TODO: Should this be an error?
            },
            Some("search") => {
                let query = msg.args[0];
                let results = seshat::default_search(&query, &self.index);
                msg.resp = Some(json!(results));
            },
            // TODO: Where does `addr` come from
            Some("stop") => self.drop_connection(*addr),
            Some("quit") => {
                // Send a close signal to all connected devices
                // NOTE: We don't remove the connections as the manager is closing anyways
                let conns = self.conns.lock()?;
                for (addr, _) in conns.iter() {
                    self.on_connection_close(&conns, *addr);
                }

                // Send the server close signal
                return self.cancel.send(())
                    .map_err(|_| Error::new(ErrorKind::ConnectionAborted, "Failed to send cancel signal"));
            },
            _ => ()
        };

        // TODO: Come up with a better interface for the routing behavior
        // NOTE: `route_message` works decently, outside of the unnecessary 'dest != sender` check
            // TODO: Would 'resolve_connection' actually fit here (they might be approaching slightly differently)
            // `route_message` also assumes a broadcast check, which is only in the 'dest' field
        self.route_message(msg, self.resolve_connection(msg.sender))
    }

    fn route_message(&mut self, msg: message::Message, dest: ResolvedDest) -> Result<(), Error> {
        if !msg.dest.broadcast.unwrap_or(false) {
            // Produce a list of the connection sinks that we want to send the message to
            // NOTE: This allows us to turn the 'dest' field into an array
            let mut send_queue = Vec::new();

            // Add the specified destination device to the queue
            if let Some(dest) = dest {
                let (_, ref sink) = self.connections.lock().unwrap()[&dest];
                send_queue.push((sink.clone(), false));
                debug!("Sending message to {:?}", dest);

                // Send an ack message to the original sender if desired
                if let Some(sender) = self.resolve_connection(msg.sender) {
                    if sender != dest {
                        let (_, ref sink) = self.connections.lock().unwrap()[&sender];
                        send_queue.push((sink.clone(), true));
                        debug!("Sending ack to {:?}", sender);
                    }
                }
            }

            // Send the json message to every connection in the queue
            for (sink, is_ack) in &send_queue {
                let msg = msg.clone();
                if is_ack {
                    msg.action = "ack".to_string();
                }
                sink.unbounded_send(serde_json::to_value(msg))?;
            }

        // Otherwise send a broadcast message to all connections
        } else {
            let msg = serde_json::to_value(msg);

            // TODO: I should add a flag to prevent looping sends
            for (addr, (_, ref sink)) in self.connections.lock().unwrap() {
                debug!("Broadcasting message to {:?}", addr);
                sink.unbounded_send(msg.clone())?;
            }
        }

        Ok(())
    }
}

impl networking::BasicServer for DeviceManager {
    fn handle_request(&mut self, msg: serde_json::Value, addr: &SocketAddr) -> Result<(), Error> {
        let mut msg: message::Message = serde_json::from_value(msg)?;
        debug!("Got {:?} from {:?}", msg, addr);

        // 1) Append the current device addr to the route array
        // 2) Set the sender's addr value if not already set
        msg.route.push(self.device_addr.unwrap());
        if msg.sender.addr.is_none() {
            msg.sender.addr = self.device_addr;
        }

        // TODO: It may be beneficial to hardcode some actions into the device manager
        // NOTE: This would be better served for notifications, as response code is tricky
        match self.resolve_destination(msg.dest) {
            None => self.handle_message(msg)?,
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
        self.connections.lock()?.insert(addr, (close_signal, write_signal));
        Ok(())
    }

    // TODO: Change the return type of this to `Result<(), Error>`
    fn drop_connection(&self, addr: SocketAddr) {
        let mut conns = self.conns.lock().unwrap();
        self.on_connection_close(&conns, addr);
        conns.remove(&addr);
    }
}
