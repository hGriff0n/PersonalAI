
// standard imports
use std::{collections, net, sync};

// third-party imports
use tokio::sync::oneshot;

// local imports
use crate::rpc;

//
// Implementation
//

struct InFlightMessage {
    pub server: net::SocketAddr,
    pub waiter: net::SocketAddr,
    pub continuation: oneshot::Sender<rpc::Message>,
}

pub struct MessageRouter {
    // Tracker of all messages currently in-flight and their continuation handles (for forwarding)
    in_flight: sync::Arc<sync::Mutex<collections::HashMap<String, InFlightMessage>>>,
    // Map of all messages being served by a particular address
    serving_messages: sync::Arc<sync::Mutex<collections::HashMap<net::SocketAddr, Vec<String>>>>,
    // Map of all messages that are being waited on by a particular address
    waiting_messages: sync::Arc<sync::Mutex<collections::HashMap<net::SocketAddr, Vec<String>>>>,
}

impl MessageRouter {
    pub fn new() -> Self {
        Self{
            in_flight: sync::Arc::new(sync::Mutex::new(collections::HashMap::new())),
            serving_messages: sync::Arc::new(sync::Mutex::new(collections::HashMap::new())),
            waiting_messages: sync::Arc::new(sync::Mutex::new(collections::HashMap::new())),
        }
    }

    pub fn wait_for_message(&self, msg_id: String, from: net::SocketAddr, to: net::SocketAddr)
        -> oneshot::Receiver<rpc::Message>
    {
        let (send, rec) = oneshot::channel();

        // Register the message as in-flight
        self.in_flight
            .lock()
            .unwrap()
            .insert(msg_id.clone(), InFlightMessage{server: to, waiter: from, continuation: send});

        // Add a notification that the server is currently processing the message
        // This enables dropping the message with an error if we lose the server connection
        self.serving_messages
            .lock()
            .unwrap()
            .entry(to)
            .or_insert(Vec::new())
            .push(msg_id.clone());

        // Add a notification that the client is waiting on the message to be processed (see above)
        self.waiting_messages
            .lock()
            .unwrap()
            .entry(from)
            .or_insert(Vec::new())
            .push(msg_id);

        rec
    }

    pub fn drop_client(&self, client: net::SocketAddr) -> Result<(), std::io::Error> {
        // Drop all `Sender` handles for messages that this client is handling
        // This has the effect of immediately completing any forwarding requests with an Error
        // NOTE: I could also get the same effect by sending out the error message here
        if let collections::hash_map::Entry::Occupied(o) = self.serving_messages.lock().unwrap().entry(client) {
            let (_, msgs) = o.remove_entry();
            let mut in_flight = self.in_flight.lock().unwrap();
            for msg in msgs {
                in_flight.remove(msg.as_str());
            }
        }

        // Replace the send end with a dummy channel with a dropped Receiver
        // This'll automatically cause the reporting of any "client dropped" errors when the server finishes
        // TODO: Figure out a way to pre-emptively stop the server from working on the request
        if let collections::hash_map::Entry::Occupied(o) = self.waiting_messages.lock().unwrap().entry(client) {
            let (_, msgs) = o.remove_entry();
            let mut in_flight = self.in_flight.lock().unwrap();
            for msg in msgs {
                if let Some(in_flight_msg) = in_flight.get_mut(&msg) {
                    let (send, _rec) = oneshot::channel();
                    in_flight_msg.continuation = send;
                }
            }
        }

        Ok(())
    }

    pub fn forward_message(&self, msg_id: String) -> Option<oneshot::Sender<rpc::Message>> {
        // Extract the sender end from the map
        if let Some(msg) = self.in_flight
            .lock()
            .unwrap()
            .remove(&msg_id)
        {
            // Remove the message from `serving_messages`
            if let Some(servers) = self.serving_messages
                .lock()
                .unwrap()
                .get_mut(&msg.server)
            {
                let idx = servers.iter().position(|x| *x == msg_id).unwrap();
                servers.remove(idx);
            }

            // Remove the message from `waiting_messages`
            if let Some(waiters) = self.waiting_messages
                .lock()
                .unwrap()
                .get_mut(&msg.waiter)
            {
                let idx = waiters.iter().position(|x| *x == msg_id).unwrap();
                waiters.remove(idx);
            }

            Some(msg.continuation)

        } else {
            None
        }
    }
}
