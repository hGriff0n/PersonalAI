
// standard imports
use std::{collections, net, sync};

// third-party imports
use failure;

// local imports
use crate::errors;
use super::service;
use super::types;
use crate::protocol;

//
// Implementation
//

/*
 * This class is meant to handle the process of receiving an rpc handle request and dispatching
 * it to whatever function was registered for that handle. The class is set up to allow for
 * the dispatch function to forward the message on to some other node if necessary
 */
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

    // This function doesn't actually return an "error" since any errors should be reported to the client
    // We instead use the 'Error' case to represent that no response should be sent
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
            .or_else(||
                Some(Box::new(futures::future::err(
                    errors::Error::from(errors::ErrorKind::RpcError(rpc_call.call.to_string())))))
            )
            .unwrap()
            // Transform any error in the handler into an error message
            .or_else(|err| {
                let err: &dyn failure::Fail = &err;
                let error_msg = types::ErrorMessage{
                    error: format!("{}", err),
                    chain: err.iter_causes().map(|cause| format!("{}", cause)).collect()
                };
                let error_send =
                    <protocol::JsonProtocol as protocol::RpcSerializer>::to_value(error_msg).unwrap();
                futures::future::ok(Some(error_send))
            })
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
    fn register(&self, fn_name: &str, callback: Box<types::Function<protocol::JsonProtocol>>)
        -> Option<errors::RegistrationError>
    {
        match self.handles
            .write()
            .unwrap()
            .entry(fn_name.to_string())
        {
            std::collections::hash_map::Entry::Vacant(entry) => { entry.insert(sync::Arc::new(callback)); None },
            _ => Some(errors::RegistrationError::handle_already_exists(fn_name))
        }
    }

    fn unregister(&self, fn_name: &str) -> Option<std::sync::Arc<Box<types::Function<protocol::JsonProtocol>>>> {
        self.handles
            .write()
            .unwrap()
            .remove(fn_name)
    }
}
