
// #[macro_use] extern crate log;
mod protocol;
#[macro_use]
mod rpc;

// services
mod fortune_service;

// macro imports
// use serde_json::json;

// standard imports
use std::net;

// third-party imports
use tokio::prelude::*;

// local imports


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
    rpc_dispatcher
        .add_service(fortune_service::FortuneService::new())
        ;

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

            // Construct communication channels between the read and write ends
            // This segments the control flow on the two ends, making the stream processing a little nicer
            let (sender, receiver) = futures::sync::mpsc::unbounded();
            // TODO: Add in close channel? Closing it on the client's end automatically closes it on the server's end

            // Receive a message from the connection and dispatch it to the rpc service
            let rpc_dispatcher = dispatcher.clone();
            let read_action = reader
                .for_each(move |msg| {
                    // TODO: Figure out how to make this asynchronous?
                    // Marshal the call off to the rpc dispatcher
                    if let Some(msg) = rpc_dispatcher.dispatch(<P as protocol::RpcSerializer>::from_value(msg)?) {
                        // If a response was produced send it back to the caller
                        sender
                            .unbounded_send(msg)
                            .map_err(|_err|
                                std::io::Error::new(std::io::ErrorKind::Other, "Failed to send message through pipe"))

                    } else {
                        Ok(())
                    }
                })
                .map(|_| ())
                .map_err(|err| eprintln!("Error: {:?}", err));

            // Reformat rpc responses and send them back on the line
            let write_action = receiver
                .map(move |msg| <P as protocol::RpcSerializer>::to_value(msg).unwrap())
                .forward(writer)
                .map(|_| ())
            .map_err(|err| eprintln!("socket write error: {:?}", err));

            // Spawn the actions in tokio
            let action = read_action
                .select2(write_action)
                .map(move |_| println!("Closed connection with {:?}", addr))
                .map_err(|_| eprintln!("close connection error"));
            tokio::spawn(action)
        });

    tokio::run(server);
}
