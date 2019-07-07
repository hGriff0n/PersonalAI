
// standard imports
use std::{collections, net, sync};

// third-party imports

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
                String, Box<types::Function<protocol::JsonProtocol>>>>>,
}

impl Dispatcher {
    pub fn new() -> Self {
        Self{
            handles: sync::Arc::new(sync::RwLock::new(collections::HashMap::new())),
        }
    }

    // TODO: Integrate this with tokio/futures better
    pub fn dispatch(&self, mut rpc_call: types::Message, caller: net::SocketAddr) -> Option<types::Message> {
        match self.handles
            .read()
            .unwrap()
            .get(&rpc_call.call)
            .and_then(|handle| Some(handle(caller, rpc_call.clone())))
        {
            // Call succeded, no response
            Some(Ok(None)) => None,

            // Call succeded, repsonse
            Some(Ok(Some(resp))) => {
                rpc_call.resp = Some(resp);
                Some(rpc_call)
            },

            // Call failed
            Some(Err(_err)) => {
                rpc_call.resp = Some(
                    <protocol::JsonProtocol as protocol::RpcSerializer>::to_value(
                        "Error: Error in rpc handler").unwrap());
                Some(rpc_call)
            },

            // Handle not registered
            None => {
                rpc_call.resp = Some(
                    <protocol::JsonProtocol as protocol::RpcSerializer>::to_value(
                        "Error: Invalid rpc call").unwrap());
                Some(rpc_call)
            }
        }
    }
}

impl service::Registry<protocol::JsonProtocol> for Dispatcher {
    fn register(&self, fn_name: &str, callback: Box<types::Function<protocol::JsonProtocol>>) -> bool {
        match self.handles
            .write()
            .unwrap()
            .entry(fn_name.to_string())
        {
            std::collections::hash_map::Entry::Vacant(entry) => { entry.insert(callback); true },
            _ => false
        }
    }

    // Temp method for initial testing of registry service integration
    fn can_register_handle(&self, fn_name: &str) -> bool {
        !self.handles.read().unwrap().contains_key(fn_name)
    }
}
