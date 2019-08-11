
// standard imports
use std::{collections, net, sync};

// third-party imports
use serde_json::json;

// local imports
use super::service;
use super::types;
use crate::protocol;

//
// Implementation
//

#[derive(Clone)]
pub struct Dispatcher {
    handles: sync::Arc<
        sync::RwLock<
            collections::HashMap<
                String, sync::Arc<Box<types::Function<protocol::JsonProtocol>>>>>>,
}

impl Dispatcher {
    pub fn new() -> Self {
        Self{
            handles: sync::Arc::new(sync::RwLock::new(collections::HashMap::new())),
        }
    }

    pub fn dispatch(&self, mut rpc_call: types::Message, caller: net::SocketAddr)
        -> impl futures::Future<Item=types::Message, Error=()>
    {
        // Extract the handle from the handles map, making sure we release the RWLock before calling it
        // This is necessary for `register_app`, etc. as they require write access to the handles map
        // TODO: This will eventually just be the `self.handles`
        let handle = self.handles
            .read()
            .unwrap()
            .get(&rpc_call.call)
            .and_then(|handle| Some(handle.clone()));

        // Now that we don't hold the RWLock, it's safe to call the handle
        use futures::future::Future;
        handle
            // Call the registerd function if one was found
            .and_then(|handle| Some(handle(caller, rpc_call.clone())))
            // If no function was registered, produce an error indicating it
            .or_else(|| Some(Box::new(futures::future::ok(Some(json!({"error": "invalid rpc call"}))))))
            .unwrap()
            // Transform any error in the handler into an error message
            .or_else(|_err| futures::future::ok(Some(json!({"error": "error in rpc handler"}))))
            // Since there are no "errors" in this result
            // We take over the 'Error' case to represent cases where a response shouldn't be sent
            // This allows us to unwrap the `Message` out of the Option
            .and_then(|resp| match resp {
                None => futures::future::err(()),
                resp => {
                    rpc_call.resp = resp;
                    futures::future::ok(rpc_call)
                }
            })
    }
}

impl service::Registry<protocol::JsonProtocol> for Dispatcher {
    fn register(&self, fn_name: &str, callback: Box<types::Function<protocol::JsonProtocol>>) -> bool {
        match self.handles
            .write()
            .unwrap()
            .entry(fn_name.to_string())
        {
            std::collections::hash_map::Entry::Vacant(entry) => { entry.insert(sync::Arc::new(callback)); true },
            _ => false
        }
    }

    fn unregister(&self, fn_name: &str) -> Option<std::sync::Arc<Box<types::Function<protocol::JsonProtocol>>>> {
        self.handles
            .write()
            .unwrap()
            .remove(fn_name)
    }
}
