extern crate futures;
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
    // I don't think I'll need to have an infinite back-and-forth communication
        // Receive play, send to a different process (I think do need to send a "done" message though)
        // Only reason would be to maintain state between negotiated communications
    // Send events back and forth, modify behavior based on the event
// Once I have this implementation done, develop a python bridge package
// Transition over to getting the modalities to work on the individual channel
// Change the dispatch to a separate app, queried by this
// Develop a tool to automatically launch components/add on the fly
// I'll also work on registering modalities with the python work

fn main() {
    let addr = "127.0.0.1:6142".parse::<SocketAddr>().unwrap();
    let listener = TcpListener::bind(&addr).unwrap();

    // TODO: See if I can write this similar to the server implementation
    let server = listener.incoming().for_each(|conn| {
        // Split the connection into reader and writer
        // Maddeningly, `conn.split` produces `(reader, writer)`
        let (writer, reader) = length_delimited::Framed::new(conn).split();
        let writer = WriteJson::<_, Value>::new(writer);
        let reader = ReadJson::<_, Value>::new(reader);

        // Setup the stop channel
        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = comm::FutureChannel::new(rx);

        // Setup the communication channel
        let (sink, source) = std::sync::mpsc::channel::<Value>();
        let source = comm::FutureChannel::new(source);

        // Define the reader action
        let read_action = reader.for_each(move |msg| {
                println!("GOT: {:?}", msg);
                Ok(sink.send(msg).unwrap())
            });

        // Define the writer action
        let write_action = writer.send_all(
            source.transform(move |mut msg| {
                msg["resp"] = json!("World");

                // Temporarily restrict the communication to a single call-response
                tx.send(()).unwrap();

                msg
            }));

        // Combine the actions into one "packet" for registration with tokio
        let action = read_action
            .select2(write_action)
            .select2(cancel)                                // NOTE: This needs to come last in order for it to work
            .map(|_| {})
            .map_err(|_| ());                               // NOTE: I'm ignoring all errors for now

        // Finally spawn the connection
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
