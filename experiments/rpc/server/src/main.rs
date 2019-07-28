
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
    pub write_queue: futures::sync::mpsc::UnboundedSender<rpc::Message>,

    // We need mutex in give interior mutability without sacrificing `Send + Sync`
    // We use `Option` to satisfy the borrow checker as `close_signal.send` moves the sender
    close_signal: std::sync::Arc<std::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,

    // TODO: Can't accept `FnOnce` because "cannot move out of borrowed content"?
    exit_callbacks: std::sync::Arc<std::sync::RwLock<Vec<Box<Fn() -> Result<(), std::io::Error> + Send + Sync>>>>,
}

impl Client {
    pub fn new(close_signal: tokio::sync::oneshot::Sender<()>, write_queue: futures::sync::mpsc::UnboundedSender<rpc::Message>)
        -> Self
    {
        Self{
            write_queue: write_queue,
            close_signal: std::sync::Arc::new(std::sync::Mutex::new(Some(close_signal))),
            exit_callbacks: std::sync::Arc::new(std::sync::RwLock::new(Vec::new())),
        }
    }

    pub fn send_close_signal(&self) {
        if let Some(signal) = self.close_signal
            .lock()
            .unwrap()
            .take()
        {
            let _ = signal.send(());
        }
    }

    // Exit callback interface
    pub fn on_exit<F>(&self, func: F)
        where F: Fn() -> Result<(), std::io::Error> + Send + Sync + 'static
    {
        self.exit_callbacks
            .write()
            .unwrap()
            .push(Box::new(func))
    }

    pub fn run_exit_callbacks(&self) -> Result<(), std::io::Error> {
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

pub struct ClientTracker {
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

    // Client tracking interface (add/get/del)
    pub fn connect_client(
        &self,
        addr: net::SocketAddr,
        write_queue: futures::sync::mpsc::UnboundedSender<rpc::Message>,
        close_signal: tokio::sync::oneshot::Sender<()>
    )
        -> std::sync::Arc<Client>
    {
        let client = std::sync::Arc::new(Client::new(close_signal, write_queue));
        self.active_clients
            .write()
            .unwrap()
            .insert(addr, client.clone());
        client
    }

    pub fn get_client(&self, addr: net::SocketAddr) -> Option<std::sync::Arc<Client>> {
        self.active_clients
            .read()
            .unwrap()
            .get(&addr)
            .and_then(|client| Some(client.clone()))
    }

    pub fn drop_client(&self, addr: net::SocketAddr) -> Result<(), std::io::Error> {
        if let Some(client) = self.active_clients
                                  .write()
                                  .unwrap()
                                  .remove(&addr)
        {
            // Try to send the close signal, in case this is called outside of `serve`
            client.send_close_signal();
            client.run_exit_callbacks()

        } else {
            Ok(())
        }
    }
}

pub struct MessageRouter {
    waiting_messages: std::sync::Arc<
        std::sync::Mutex<
            std::collections::HashMap<String, tokio::sync::oneshot::Sender<rpc::Message>>>>,
}

impl MessageRouter {
    pub fn new() -> Self {
        Self{
            waiting_messages: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }

    // TODO: Drop by waiting app/sending app
    pub fn forward_message(&self, msg_id: String, _from: net::SocketAddr, _to: net::SocketAddr)
        -> tokio::sync::oneshot::Receiver<rpc::Message>
    {
        let (send, rec) = tokio::sync::oneshot::channel();

        self.waiting_messages
            .lock()
            .unwrap()
            .insert(msg_id, send);

        rec
    }

    pub fn take_sender(&self, msg_id: String) -> Option<tokio::sync::oneshot::Sender<rpc::Message>> {
        self.waiting_messages
            .lock()
            .unwrap()
            .remove(&msg_id)
    }
}

mod registration_service {
    use serde::{Serialize, Deserialize};
    use crate::protocol;
    use crate::rpc;
    use crate::rpc::Registry;

    pub struct RegistrationService {
        registry: std::sync::Arc<rpc::dispatch::Dispatcher>,
        clients: std::sync::Arc<crate::ClientTracker>,
        router: std::sync::Arc<crate::MessageRouter>,
    }

    impl RegistrationService {
        pub fn new(registry: std::sync::Arc<rpc::dispatch::Dispatcher>,
                   clients: std::sync::Arc<crate::ClientTracker>,
                   router: std::sync::Arc<crate::MessageRouter>)
            -> Self
        {
            Self{
                registry: registry,
                clients: clients,
                router: router
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
            let mut resp = RegisterAppResponse{ registered: Vec::new() };

            // Get the client tracker object
            let server = match self.clients.get_client(caller) {
                Some(server) => server,
                _ => Err(
                    std::io::Error::new(
                        std::io::ErrorKind::ConnectionReset,
                        format!("No registered client for {}", caller)))?
            };

            // Register a handler for the specified functions to forward the message to the app server
            // TODO: How do I get responses from the app server back into this callback?
            for handle in args.handles {
                let msg_forwarding = server.write_queue.clone();
                let router = self.router.clone();

                // TODO: Is there a way I could make writing this callback simpler?
                let callback = move |app_caller: std::net::SocketAddr, msg: rpc::Message|
                    -> Result<Option<<protocol::JsonProtocol as protocol::RpcSerializer>::Message>, std::io::Error>
                {
                    // TODO: This will be gotten from a future/callback after the send below
                    let msg_id = msg.msg_id.clone();

                    // TODO: Should we add information about the original caller to the Message schema?
                    msg_forwarding.unbounded_send(msg.clone())
                        .map_err(|_err| std::io::Error::new(
                            std::io::ErrorKind::ConnectionAborted,
                            format!("Receiving end for client {} dropped", caller)))?;

                    // TODO: Move this into the 'futures' framework
                    // The `try_recv` is necessary to type it currently, but doesn't give the behavior we want
                    let resp = router.forward_message(msg_id.clone(), caller, app_caller)
                        .try_recv()
                        .map_err(|_err| std::io::Error::new(
                            std::io::ErrorKind::ConnectionAborted,
                            format!("Client disconnected while waiting for response to {}", msg_id)
                        ))?;
                    Ok(Some(<protocol::JsonProtocol as protocol::RpcSerializer>::to_value(resp)?))
                };

                if self.registry.register_fn(handle.as_str(), callback) {
                    resp.registered.push(handle);
                }
            }

            // Add a final callback to de-register all the handles we just added when the client exits
            // TODO: Not sure about error handling here
            let registered = resp.registered.clone();
            let reg = self.registry.clone();
            server.on_exit(move ||
                Ok(for handle in &registered {
                    if let Some(callback) = reg.unregister(handle.as_str()) {
                        if std::sync::Arc::strong_count(&callback) > 1 {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                format!("Strong references held to dispatcher for app callback `{}` at deregistration", handle)
                            ));
                        }
                    }
                }));

            resp
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
    let msg_router = std::sync::Arc::new(MessageRouter::new());
    let rpc_dispatcher = std::sync::Arc::new(rpc::dispatch::Dispatcher::new());

    // Create and register services in the dispatcher
    experimental_service::ExperimentalService::new()
        .register_endpoints(&*rpc_dispatcher)
        .unwrap_or_else(|err| panic!(err));

    // NOTE: Can wrap this in a macro, not sure if good => add_service!($dispatcher:ident $service:expr);
    fortune_service::FortuneService::new()
        .register_endpoints(&*rpc_dispatcher)
        .unwrap_or_else(|err| panic!(err));

    registration_service::RegistrationService::new(rpc_dispatcher.clone(), client_tracker.clone(), msg_router.clone())
        .register_endpoints(&*rpc_dispatcher)
        .unwrap_or_else(|err| panic!(err));

    // We've constructed our rpc server
    // Now let the user break it
    serve(rpc_dispatcher, client_tracker, msg_router, addr);
}

// TODO: Improve error handling?
fn serve(dispatcher: std::sync::Arc<rpc::dispatch::Dispatcher>,
         client_tracker: std::sync::Arc<ClientTracker>,
         msg_router: std::sync::Arc<MessageRouter>,
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
            let client = client_tracker.connect_client(peer_addr, sender, signal);
            let drop_client = client.clone();

            // TODO: Add in `register_new_connection` type callback?

            // TODO: Figure out how to make this asynchronous
            // Receive a message from the connection and dispatch it to the rpc service
            let rpc_dispatcher = dispatcher.clone();
            let router = msg_router.clone();
            let read_action = reader
                .for_each(move |msg| {
                    let rpc_msg: rpc::Message = <P as protocol::RpcSerializer>::from_value(msg)?;

                    // Check if there is any forwarding setup for the message we just received
                    // NOTE: This part doesn't technically need to be asynchronous
                    // Depends on how the dispatch asynchony is implemented
                    if let Some(sender) = router.take_sender(rpc_msg.msg_id.clone()) {
                        return sender.send(rpc_msg.clone())
                            .map_err(|_err| std::io::Error::new(
                                std::io::ErrorKind::UnexpectedEof,
                                format!("Failed to forward message: {:?}", rpc_msg)
                            ))
                    }

                    // TODO: Figure out how to make this asynchronous?
                    // NOTE: This needs to be asynchonous
                    // Marshal the call off to the rpc dispatcher
                    if let Some(msg) = rpc_dispatcher.dispatch(rpc_msg, peer_addr) {
                        // If a response was produced send it back to the caller
                        client.write_queue
                            .unbounded_send(msg)
                            .map_err(|_err|
                                std::io::Error::new(std::io::ErrorKind::Other, "Failed to send message through pipe"))?;

                    }

                    Ok(())
                })
                .map(|_| ())
                .map_err(|err| eprintln!("Error: {:?}", err));

            // Reformat rpc responses and send them down the line to the client
            let write_action = receiver
                .map(move |msg| <P as protocol::RpcSerializer>::to_value(msg).unwrap())
                .forward(writer)
                .map(|_| ())
                .map_err(|err| eprintln!("socket write error: {:?}", err));

            // Ensure the close signal gets sent after read_action|write_action finishes
            // This avoids some errors with close_action resulting from the signal not being sent
            let communication_action = read_action
                .select(write_action)
                .map(move |_|
                    drop_client.send_close_signal()
                );

            // Catch any errors by close_channel so that we can actually print them
            let close_action = close_channel
                .map_err(|err| eprintln!("close connection error: {:?}", err));

            // Drop the connection from the client tracker
            let client_dropper = client_tracker.clone();
            let action = communication_action
                .select2(close_action)
                .map(move |_| {
                    client_dropper.drop_client(peer_addr)
                        .err()
                        .and_then(|err| Some(eprintln!("drop client error: {:?}", err)));
                    println!("Closed connection with {:?}", peer_addr)
                })
                .map_err(|_| eprintln!("unknown error occurred"));

            // Spawn the actions in tokio
            tokio::spawn(action)
        });

    tokio::run(server);
}
