// extern crate futures;
extern crate tokio;
// extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_serde_json;

#[macro_use]
extern crate serde_json;

mod comm;

// This program acts as the interaction manager for the individual device,
// Collecting and dispatching requests to the global server from modalities
// While maintaining and handling system level state/operations

use std::net::SocketAddr;

use tokio::prelude::*;
use tokio::net::TcpListener;
use tokio_io::codec::length_delimited;

use serde_json::Value;
use tokio_serde_json::*;

// Adapt this with the server to enable easy two-way communication
    // Send events back and forth, modify behavior based on the event
// Once I have this implementation done, develop a python bridge package
// Transition over to getting the modalities to work on the individual channel
// Change the dispatch to a separate app, queried by this
// Develop a tool to automatically launch components/add on the fly
// I'll also work on registering modalities with the python work

fn main() {
    let addr = "127.0.0.1:6142".parse::<SocketAddr>().unwrap();
    let listener = TcpListener::bind(&addr).unwrap();

    let server = listener.incoming().for_each(|conn| {
        // Split the connection into reader and writer
        let (writer, reader) = length_delimited::Framed::new(conn).split();
        let writer = WriteJson::<_, Value>::new(writer);
        let reader = ReadJson::<_, Value>::new(reader);

        // Setup the stop channel
        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = comm::Canceller{ rx: rx };

        // Produce the action
        let action = reader.map(move |msg| {
                tx.send(()).unwrap();
                println!("GOT: {:?}", msg);
                msg
            })
            .forward(writer)
            .select2(cancel)
            .map(|_| {})
            .map_err(|err| println!("error"));          // TODO: Why can't I print the error ?

        tokio::spawn(action);
        Ok(())
    })
    .map_err(|err| {
        println!("Server error: {:?}", err)
    });

    // Start the server and tokio runtime
    tokio::run(server);
}

// API Documentation:
//  tokio-serde-json: https://github.com/carllerche/tokio-serde-json
