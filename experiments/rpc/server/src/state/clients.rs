
// standard imports
use std::{collections, net, sync};

// third-party imports
use futures::sync::mpsc;
use log::*;
use tokio::sync::oneshot;

// local imports
use crate::errors;
use crate::rpc;

//
// Implementation
//

/*
 * Structure which holds all information describing a specific connection to the device manager
 *
 * If messages need to be written to the connection, the should be put on the `write_queue`
 */
pub struct Client {
    pub write_queue: mpsc::UnboundedSender<rpc::Message>,

    // We need mutex in give interior mutability without sacrificing `Send + Sync`
    // We use `Option` to satisfy the borrow checker as `close_signal.send` moves the sender
    close_signal: sync::Arc<sync::Mutex<Option<oneshot::Sender<()>>>>,

    // TODO: Can't accept `FnOnce` because "cannot move out of borrowed content"?
    exit_callbacks: sync::Arc<sync::RwLock<Vec<Box<dyn Fn() -> Result<(), errors::Error> + Send + Sync>>>>,
}

impl Client {
    pub fn new(close_signal: oneshot::Sender<()>, write_queue: mpsc::UnboundedSender<rpc::Message>)
        -> Self
    {
        Self{
            write_queue: write_queue,
            close_signal: sync::Arc::new(sync::Mutex::new(Some(close_signal))),
            exit_callbacks: sync::Arc::new(sync::RwLock::new(Vec::new())),
        }
    }

    // Helper method to send the signal to drop the client connection
    // This ensures that the connection is only ever dropped once
    pub fn send_close_signal(&self) {
        if let Some(signal) = self.close_signal
            .lock()
            .unwrap()
            .take()
        {
            let _ = signal.send(());
        }
    }

    // Add a function to be run when the client is shutdown
    pub fn on_exit<F>(&self, func: F)
        where F: Fn() -> Result<(), errors::Error> + Send + Sync + 'static
    {
        self.exit_callbacks
            .write()
            .unwrap()
            .push(Box::new(func))
    }

    // Run all of the registered callbacks
    // This ensures that the callbacks are only ever run once
    pub fn run_exit_callbacks(&self) -> Result<(), errors::Error> {
        let mut callbacks = self.exit_callbacks
            .write()
            .unwrap();

        // Run all callbacks returning the first error we encounter
        // NOTE: We don't immediately return on errors as:
            // 1) Callbacks should not be depending on the ordering callbacks are run anyways
            // 2) Not calling a callback may leave the system in an invalid state which'll produce future errors
        // If we have errors in multiple callbacks, we always return the first error though
        let ret = match callbacks.iter()
                       .filter_map(|callback| callback().err())
                       .next()
        {
            Some(err) => Err(err),
            _ => Ok(())
        };

        // Clear the callbacks so we only run through them once (just in case)
        callbacks.clear();
        ret
    }
}

/*
 * Structure which tracks all connnections and the socket address they are connected on
 */
pub struct ClientTracker {
    active_clients: sync::Arc<sync::RwLock<collections::HashMap<net::SocketAddr, sync::Arc<Client>>>>,
}

impl ClientTracker {
    pub fn new() -> Self {
        Self{
            active_clients: sync::Arc::new(sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    // Add a client to the tracking map
    pub fn connect_client(
        &self,
        addr: net::SocketAddr,
        write_queue: mpsc::UnboundedSender<rpc::Message>,
        close_signal: oneshot::Sender<()>
    )
        -> sync::Arc<Client>
    {
        let client = sync::Arc::new(Client::new(close_signal, write_queue));
        self.active_clients
            .write()
            .unwrap()
            .insert(addr, client.clone());
        debug!("Registered new client connection on {}", addr);
        client
    }

    // Retrieve the client that is connected on the passed socket address
    pub fn get_client(&self, addr: net::SocketAddr) -> Option<sync::Arc<Client>> {
        self.active_clients
            .read()
            .unwrap()
            .get(&addr)
            .and_then(|client| Some(client.clone()))
    }

    // Close the client on the specific socket address
    pub fn drop_client(&self, addr: net::SocketAddr) -> Result<(), errors::Error> {
        if let Some(client) = self.active_clients
                                  .write()
                                  .unwrap()
                                  .remove(&addr)
        {
            debug!("Dropping client connection to {}", addr);

            // Try to send the close signal, in case this is called outside of `serve`
            client.send_close_signal();
            client.run_exit_callbacks()

        } else {
            Ok(())
        }
    }
}
