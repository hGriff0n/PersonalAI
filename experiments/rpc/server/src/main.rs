
// #[macro_use] extern crate log;
mod protocol;
#[macro_use]
mod rpc;
mod state;

// services
mod experimental_service;
mod fortune_service;
mod registration_service;

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

//
// Server running code
//

fn main() {
    let addr = "127.0.0.1:6142".parse::<net::SocketAddr>()
        .expect("Failed to parse hardcoded socket address");

    // let device_manager = DeviceManager::new();
    let client_tracker = std::sync::Arc::new(state::clients::ClientTracker::new());
    let msg_router = std::sync::Arc::new(state::routing::MessageRouter::new());
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
         client_tracker: std::sync::Arc<state::clients::ClientTracker>,
         msg_router: std::sync::Arc<state::routing::MessageRouter>,
         addr: std::net::SocketAddr)
{
    // Current protocols don't require state, so we currently access it statically
    // TODO: Need a way to ensure we're all using the same protocol (across service definitions, etc.)
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

            // Receive a message from the connection and dispatch it to the rpc service
            let rpc_dispatcher = dispatcher.clone();
            let router = msg_router.clone();
            let read_action = reader
                .for_each(move |msg| {
                    let rpc_msg: rpc::Message = <P as protocol::RpcSerializer>::from_value(msg)?;

                    // Check if there is any forwarding setup for the message we just received
                    if let Some(sender) = router.forward_message(rpc_msg.msg_id.clone()) {
                        let rpc_msg_id = rpc_msg.msg_id.clone();
                        return sender.send(rpc_msg)
                            .map_err(|_err| std::io::Error::new(
                                std::io::ErrorKind::UnexpectedEof,
                                format!("Client disconnected while waiting for response to {}", rpc_msg_id)
                            ));
                    }

                    // Marshal the call off to the rpc dispatcher (asynchronous)
                    let client = client.clone();
                    let dispatch_fn = rpc_dispatcher.dispatch(rpc_msg, peer_addr)
                        .and_then(move |resp|
                            client.write_queue
                                .unbounded_send(resp)
                                .map_err(|_err|
                                    eprintln!("async dispatch error: Failed to send message to client")));
                    tokio::spawn(dispatch_fn);
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
            let in_flight_dropper = msg_router.clone();
            let action = communication_action
                .select2(close_action)
                .map(move |_| {
                    // Deregister the client
                    client_dropper.drop_client(peer_addr)
                        .err()
                        .and_then(|err| Some(eprintln!("drop client error: {:?}", err)));

                    // And drop any messages that it's involved in servicing
                    in_flight_dropper.drop_client(peer_addr)
                        .err()
                        .and_then(|err| Some(eprintln!("drop client messages error: {:?}", err)));

                    println!("Closed connection with {:?}", peer_addr)
                })
                .map_err(|_| eprintln!("unknown error occurred"));

            // Spawn the actions in tokio
            tokio::spawn(action)
        });

    tokio::run(server);
}
