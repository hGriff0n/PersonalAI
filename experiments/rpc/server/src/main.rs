
// #[macro_use] extern crate log;
mod protocol;
#[macro_use]
mod rpc;

// services
mod experimental_service;
mod fortune_service;
// mod registration_service;

// macro imports
// use serde_json::json;

// standard imports
use std::net;

// third-party imports
use tokio::prelude::*;

// local imports
use crate::rpc::Service;

//
// Implementation
//

pub struct Client {
    close_signal: std::sync::Arc<std::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
    write_queue: futures::sync::mpsc::UnboundedSender<rpc::Message>,
}

impl Client {
    pub fn send_close_signal(&self) {
        if let Some(signal) = self.close_signal
            .lock()
            .unwrap()
            .take()
        {
            let _ = signal.send(());
        }
    }
}

pub struct ClientTracker {
    #[allow(dead_code)]
    active_clients: std::sync::Arc<
        std::sync::RwLock<
            std::collections::HashMap<net::SocketAddr, std::sync::Arc<Client>>>>,
}

impl ClientTracker {
    pub fn new() -> Self {
        Self{
            active_clients: std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub fn new_client(&self, addr: net::SocketAddr, write_queue: futures::sync::mpsc::UnboundedSender<rpc::Message>, close_signal: tokio::sync::oneshot::Sender<()>)
        -> std::sync::Arc<Client>
    {
        let client = std::sync::Arc::new(Client{write_queue: write_queue, close_signal: std::sync::Arc::new(std::sync::Mutex::new(Some(close_signal)))});
        self.active_clients
            .write()
            .unwrap()
            .insert(addr, client.clone());
        client
    }

    pub fn drop_client(&self, addr: net::SocketAddr) -> Option<std::sync::Arc<Client>> {
        self.active_clients
            .write()
            .unwrap()
            .remove(&addr)
    }
}

mod registration_service {
    use serde::{Serialize, Deserialize};
    use crate::protocol;
    use crate::rpc;
    use crate::rpc::Registry;

    pub struct RegistrationService {
        registry: std::sync::Arc<rpc::dispatch::Dispatcher>,

        #[allow(dead_code)]
        clients: std::sync::Arc<crate::ClientTracker>,
    }

    impl RegistrationService {
        pub fn new(registry: std::sync::Arc<rpc::dispatch::Dispatcher>, clients: std::sync::Arc<crate::ClientTracker>)
            -> Self
        {
            Self{
                registry: registry,
                clients: clients,
            }
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
            // let mut client = self.server_state.client(caller);

            let mut registered = Vec::new();
            for handle in args.handles {
                // TODO: How do we inform the read end to send this message back to the caller?
                // let write_queue = client.write_queue();
                // let handle_fn = handle.clone();
                let callback = move |_caller: std::net::SocketAddr, msg: rpc::Message| {
                    // TODO: Should we add information about the original caller to the Message schema?
                    // write_queue.send(msg);
                    // TODO: What's the best way of returning the response from the client
                        // None + Add signal to send message from read of handle to the caller
                        // `wait_on_message` future that resolves once the app returns it's response
                    Ok(Some(<protocol::JsonProtocol as protocol::RpcSerializer>::to_value(msg)?))
                };

                if self.registry.register_fn(handle.as_str(), callback) {
                    registered.push(handle);
                }
            }

            // TODO: Not sure about error handling here
            // client.on_exit(|| for handle in registered {
            //     if let Some(callback) = self.registry.unregister(handle.as_str()) {
            //         if std::sync::Arc::strong_count(&callback) > 1 {
            //             panic!("Multiple strong references held to dispatch callback for {} at deregister", handle);
            //         }
            //     }
            // });

            RegisterAppResponse{
                registered: registered
            }
        }
    }
}


//
// Server running code
//

fn main() {
    let addr = "127.0.0.1:6142".parse::<net::SocketAddr>()
        .expect("Failed to parse hardcoded socket address");

    // let device_manager = DeviceManager::new();
    let client_tracker = std::sync::Arc::new(ClientTracker::new());
    let rpc_dispatcher = std::sync::Arc::new(rpc::dispatch::Dispatcher::new());

    // Create and register services in the dispatcher
    experimental_service::ExperimentalService::new()
        .register_endpoints(&*rpc_dispatcher)
        .unwrap_or_else(|err| panic!(err));

    // NOTE: Can wrap this in a macro, not sure if good => add_service!($dispatcher:ident $service:expr);
    fortune_service::FortuneService::new()
        .register_endpoints(&*rpc_dispatcher)
        .unwrap_or_else(|err| panic!(err));

    registration_service::RegistrationService::new(rpc_dispatcher.clone(), client_tracker.clone())
        .register_endpoints(&*rpc_dispatcher)
        .unwrap_or_else(|err| panic!(err));

    // We've constructed our rpc server
    // Now let the user break it
    serve(rpc_dispatcher, client_tracker, addr);
}

// TODO: Improve error handling?
fn serve(dispatcher: std::sync::Arc<rpc::dispatch::Dispatcher>,
         client_tracker: std::sync::Arc<ClientTracker>,
         addr: std::net::SocketAddr)
{
    // Current protocols don't require state, so we currently access it statically
    // TODO: Need a way to ensure we're all using the same protocol
    type P = protocol::JsonProtocol;

    let server = tokio::net::TcpListener::bind(&addr)
        .expect("Failed to bind server to specified sock address")
        .incoming()
        .map_err(|err| eprintln!("Failed to accept incoming connection: {:?}", err))
        .for_each(move |conn| {
            // Extract information about the specific connection
            // `peer_addr` is especially important because we'll use that as the "primary" identifier this client
            let peer_addr = conn
                .peer_addr()
                .expect("Failed to extract peer address from TcpStream");

            // Construct the communication frames
            let (reader, writer) = protocol::frame_with_protocol::<P, _, _>(
                conn, &|| tokio::codec::LengthDelimitedCodec::new());
            let writer = writer
                .sink_map_err(|err| eprintln!("error in json serialization: {:?}", err));

            // Construct channels between the read, write, and close "segments"
            // This separates the control flow into several ends, making the stream processing a little nicer
            let (sender, receiver) = futures::sync::mpsc::unbounded();
            let (signal, close_channel) = tokio::sync::oneshot::channel();

            // Now that we've set all the channels up, start tracking the new client
            let client = client_tracker.new_client(peer_addr, sender, signal);
            let client_dropper = client_tracker.clone();
            let client_double_dropper = client_tracker.clone();

            // TODO: Add in `register_new_connection` type callback?

            // Receive a message from the connection and dispatch it to the rpc service
            let rpc_dispatcher = dispatcher.clone();
            let read_action = reader
                .for_each(move |msg| {
                    // TODO: Figure out how to make this asynchronous?
                    // Marshal the call off to the rpc dispatcher
                    if let Some(msg) = rpc_dispatcher
                        .dispatch(<P as protocol::RpcSerializer>::from_value(msg)?, peer_addr)
                    {
                        // If a response was produced send it back to the caller
                        client.write_queue
                            .unbounded_send(msg)
                            .map_err(|_err|
                                std::io::Error::new(std::io::ErrorKind::Other, "Failed to send message through pipe"))?;

                    } else {
                        println!("No response");
                    }

                    Ok(())
                })
                .map(|_| ())
                .map_err(|err| eprintln!("Error: {:?}", err));

            // Reformat rpc responses and send them back on the line
            let write_action = receiver
                .map(move |msg| <P as protocol::RpcSerializer>::to_value(msg).unwrap())
                .forward(writer)
                .map(|_| ())
                .map_err(|err| eprintln!("socket write error: {:?}", err));

            // Catch any errors by close_channel so that we can actually print them
            // NOTE: Some errors we catch will still be reported as an `unknown error` somehow
                // This is likely due to the signal being dropped without having called `send`
            let close_action = close_channel
                .map(|_| ())
                .map_err(|err| eprintln!("close connection error: {:?}", err));

            // Spawn the actions in tokio
            let action = read_action
                .select(write_action)
                .map(move |_| {
                    // Ensure the close signal gets sent so we don't have any errors in close_action
                    if let Some(client) = client_dropper.drop_client(addr) {
                        client.send_close_signal();
                    }
                })
                .select2(close_action)
                .map(move |_| {
                    client_double_dropper.drop_client(addr);
                    println!("Closed connection with {:?}", addr)
                })
                .map_err(|_| eprintln!("unknown error occurred"));

            tokio::spawn(action)
        });

    tokio::run(server);
}
