
// standard imports
use std::{collections, sync};

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
    handles: sync::Arc<sync::RwLock<collections::HashMap<String, Box<types::Function<protocol::JsonProtocol>>>>>,
}

impl Dispatcher {
    pub fn new() -> Self {
        Self{
            handles: sync::Arc::new(sync::RwLock::new(collections::HashMap::new())),
        }
    }

    pub fn add_service<S: service::Service<protocol::JsonProtocol>>(self, service: S) -> Self {
        for (endpoint, callback) in service.endpoints() {
            self.handles
                .write()
                .unwrap()
                .insert(endpoint, callback);
        }
        self
    }

    // TODO: Integrate this with tokio/futures better
    pub fn dispatch(&self, mut rpc_call: types::Message) -> Option<types::Message> {
        match self.handles
            .read()
            .unwrap()
            .get(&rpc_call.call)
            .and_then(|handle| Some(handle(rpc_call.clone())))
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
