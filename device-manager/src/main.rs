// extern crate futures;
extern crate tokio;
// extern crate tokio_core;
// extern crate tokio_io;
// extern crate tokio_serde_json;

extern crate tokio_serde_json;

// This program acts as the interaction manager for the individual device,
// Collecting and dispatching requests to the global server from modalities
// While maintaining and handling system level state/operations

// TODO: I may want to use json-rpc for communication in the future
// But for the moment, I want to make sure everything works
// https://github.com/vorner/tokio-jsonrpc
// https://github.com/joshmarshall/jsonrpclib

// For the moment, I'll just pass simple json messages back and forth
// https://github.com/carllerche/tokio-serde-json

use tokio::io;
use tokio::net::TcpListener;
use tokio::prelude::*;

use std::net::SocketAddr;

// Use length delimited frames
// use tokio_io::codec::length_delimited;

use tokio_serde_json::WriteJson;

// Get tokio communication to work (https://lukesteensen.com/2016/12/getting-started-with-tokio/)
// Modify this to allow for json communication
// Adapt this with the server to enable easy two-way communication
// Once I have this imlis implementation done, develop a python bridge package
// Transition over to getting the modalities to work on thee individual channel
// Change the dispatch to a separate app, queried by this
// Develop a tool to automatically launch components/add on the fly
// I'll also work on registering modalities with the python work
fn main() {
    let addr = "127.0.0.1:6142".parse::<SocketAddr>().unwrap();
    let listener = TcpListener::bind(&addr).unwrap();

    let server = listener.incoming().for_each(|tcp| {
        // Split up the read and write halves
        let (reader, writer) = tcp.split();

        // Copy the data back to the client
        let conn = io::copy(reader, writer)
            // print what happened
            .map(|(n, _, _)| {
                println!("Wrote {} bytes!", n)
            })
            // Handle any errors
            .map_err(|err| {
                println!("IO error: {:?}", err)
            });

        // Spawn the future as a concurrent task
        tokio::spawn(conn);

        Ok(())
    })
    .map_err(|err| {
        println!("Server error: {:?}", err)
    });

    // Start the server and tokio runtime
    tokio::run(server);
    println!("Hello")
}
