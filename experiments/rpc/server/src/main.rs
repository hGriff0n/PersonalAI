
// #[macro_use] extern crate log;
mod protocol;
#[macro_use]
mod rpc;

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
    let rpc_dispatcher = rpc::dispatch::Dispatcher::new();

    // Create and register services in the dispatcher
    experimental_service::ExperimentalService::new()
        .register_endpoints(&rpc_dispatcher)
        .unwrap_or_else(|err| panic!(err));

    // NOTE: Can wrap this in a macro, not sure if good => ($dispatcher:ident $service:expr)
    fortune_service::FortuneService::new()
        .register_endpoints(&rpc_dispatcher)
        .unwrap_or_else(|err| panic!(err));

    registration_service::RegistrationService::new()
        .register_endpoints(&rpc_dispatcher)
        .unwrap_or_else(|err| panic!(err));

    // We've constructed our rpc server
    // Now let the user break it
    serve(rpc_dispatcher, addr);
}

// TODO: Improve error handling?
fn serve(dispatcher: rpc::dispatch::Dispatcher, addr: std::net::SocketAddr) {
    // Current protocols don't require state, so we currently access it statically
    // TODO: Need a way to ensure we're all using the same protocol
    type P = protocol::JsonProtocol;

    let server = tokio::net::TcpListener::bind(&addr)
        .expect("Failed to bind server to specified sock address")
        .incoming()
        .map_err(|err| eprintln!("Failed to accept incoming connection: {:?}", err))
        .for_each(move |conn| {
            let _peer = conn
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
            // TODO: This is not how the original code handles this. Might want to change
                // We can also keep the code in the read handler if chosen
            let (signal, close_channel) = tokio::sync::oneshot::channel::<()>();

            // NOTE: This is required in order to allow for sending the signal to the read action in the current impl
            let mut signal = Some(signal);

            // TODO: Add in `register_new_connection` type callback?

            // Receive a message from the connection and dispatch it to the rpc service
            let rpc_dispatcher = dispatcher.clone();
            let read_action = reader
                .for_each(move |msg| {
                    // TODO: This is called for every request/response -> bad for app handling
                        // The old code handled this by registering the 'connection' outside of this scope
                        // Handler code would then access the "state" object to get the cancel signal
                    // NOTE: This code is required in order to send the signal into the closure
                    let close_signal = signal.take();

                    // TODO: Figure out how to make this asynchronous?
                    // Marshal the call off to the rpc dispatcher
                    if let Some(msg) = rpc_dispatcher.dispatch(<P as protocol::RpcSerializer>::from_value(msg)?) {
                        // If a response was produced send it back to the caller
                        sender
                            .unbounded_send(msg)
                            .map_err(|_err|
                                std::io::Error::new(std::io::ErrorKind::Other, "Failed to send message through pipe"))?;

                    } else {
                        println!("No response");
                    }

                    // TODO: We can currently only accept one response per client (since we don't persist the signal)
                    close_signal.unwrap().send(());
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
                .select2(close_action)
                // TODO: Add in `register_close_connection` type callback?
                .map(move |_| println!("Closed connection with {:?}", addr))
                .map_err(|_| eprintln!("unknown error occurred"));

            tokio::spawn(action)
        });

    tokio::run(server);
}
