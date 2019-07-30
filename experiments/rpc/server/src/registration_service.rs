
// standard imports
use std::{net, sync};

// third-party imports
use futures::{Future, future};
use serde::{Serialize, Deserialize};

// local imports
use crate::protocol;
use crate::rpc;
use crate::rpc::Registry;
use crate::state;

//
// Implementation
//

pub struct RegistrationService {
    registry: sync::Arc<rpc::dispatch::Dispatcher>,
    clients: sync::Arc<state::clients::ClientTracker>,
    router: sync::Arc<state::routing::MessageRouter>,
}

impl RegistrationService {
    pub fn new(registry: sync::Arc<rpc::dispatch::Dispatcher>,
               clients: sync::Arc<state::clients::ClientTracker>,
               router: sync::Arc<state::routing::MessageRouter>)
        -> Self
    {
        Self{
            registry: registry,
            clients: clients,
            router: router,
        }
    }

    fn register_app_impl(&self, server: sync::Arc<state::clients::Client>, app_address: net::SocketAddr, handles: &[String])
        -> Result<Vec<String>, std::io::Error>
    {
        let mut registered = Vec::new();

        // Register a handler for the specified functions to forward the message to the app server
        for handle in handles {
            let app_msg_queue = server.write_queue.clone();
            let router = self.router.clone();

            // Create the dispatcher callback that will forward any requests on this rpc to the server app
            let callback = move |caller: net::SocketAddr, msg: rpc::Message|
                -> Box<dyn Future<
                    Item=Option<<protocol::JsonProtocol as protocol::RpcSerializer>::Message>,
                    Error=std::io::Error> + Send>
            {
                // Send the message over to the server app
                // Return immediately if an error was found
                if let Err(_err) = app_msg_queue.unbounded_send(msg.clone()) {
                    return Box::new(future::err(std::io::Error::new(
                        std::io::ErrorKind::ConnectionAborted,
                        format!("Receiving end for server {} dropped", app_address))));
                }

                // Otherwise register an entry in the forwarding table
                // And then wait on the response from the server app
                let msg_id = msg.msg_id.clone();
                let forward_resp = router.wait_for_message(msg_id.clone(), caller, app_address)
                    .map_err(move |_err| std::io::Error::new(
                        std::io::ErrorKind::ConnectionAborted,
                        format!("Server disconnected while handling request to {}", msg_id)))
                    .and_then(|resp| match <protocol::JsonProtocol as protocol::RpcSerializer>::to_value(resp) {
                        Ok(resp) => future::ok(Some(resp)),
                        Err(err) => future::err(err),
                    });
                Box::new(forward_resp)
            };

            // And then attempt to register that rpc and callback in the system dispatch table
            // NOTE: If a registration fails, we do not do any error handling at the moment
            // It is the server's responsibility to recognize that a handle was not registered
            // And to fail if that handle's registration must succeed
            if self.registry.register_fn(handle, callback) {
                registered.push(handle.to_owned());
            }
        }

        Ok(registered)
    }
}

//
// RpcService Definition
//

// TODO: Look into possibility of adding extra schema information/etc.
rpc_schema!(RegisterAppArgs {
    handles: Vec<String>
});

rpc_schema!(RegisterAppResponse {
    registered: Vec<String>
});

rpc_service! {
    RegistrationService<protocol::JsonProtocol>

    rpc register_app(self, caller, args: RegisterAppArgs) -> RegisterAppResponse {
        self.clients.get_client(caller)
            // We have a client object so let's register the handles and exit callbacks
            .and_then(|server| Some(match self.register_app_impl(server.clone(), caller, &args.handles) {
                Err(err) => future::err(err),
                Ok(registered) => {
                    let resp = RegisterAppResponse{registered: registered.clone()};

                    let reg = self.registry.clone();
                    server.on_exit(move || {
                        for handle in &registered {
                            if let Some(callback) = reg.unregister(handle.as_str()) {
                                if sync::Arc::strong_count(&callback) > 1 {
                                    return Err(std::io::Error::new(
                                        std::io::ErrorKind::Other,
                                        format!("Strong references held to dispatcher for app callback `{}` at deregistration", handle)
                                    ));
                                }
                            }
                        }
                        Ok(())
                    });

                    future::ok(resp)
                }
            }))

            // For some reason there was no registered client
            .unwrap_or_else(|| future::err(std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                format!("No registered client for {}", caller))))
    }
}
