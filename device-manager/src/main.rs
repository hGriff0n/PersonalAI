extern crate tokio;
extern crate tokio_io;
extern crate tokio_serde_json;

#[macro_use]
extern crate serde_json;

mod comm;

// This program acts as the interaction manager for the individual device,
// Collecting and dispatching requests to the global server from modalities
// While maintaining and handling system level state/operations

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, mpsc, Mutex};

use tokio::prelude::*;
use tokio::net::TcpListener;
use tokio_io::codec::length_delimited::Framed;

use serde_json::Value;
use tokio_serde_json::*;

// Adapt this with the server to enable easy two-way communication
    // Package this behavior into a common directory
        // Goal: Provide two functions to setup this automatic communication
    // Another issue is that they use different "communication" streams/methods
        // The final implementation should be able to act in both capacities
    // This may actually be a bit difficult depending on how much I want to setup automatically
        // All of that data will not be automatically visible to the caller
        // I could always provide a "setup" method/struct to encapsulate the behavior
    // Send events back and forth, modify behavior based on the event
// Get working cross-device communication (move away from home ip)
    // Figure out how to implement discovery so I don't have to hardcode paths
// Once I have this implementation done, develop a python bridge package
// Transition over to getting the modalities to work on the individual channel
// Change the dispatch to a separate app, queried by this
// Develop a tool to automatically launch components/add on the fly
// I'll also work on registering modalities with the python work

fn main() {
    let addr = "127.0.0.1:6142".parse::<SocketAddr>().unwrap();
    let listener = TcpListener::bind(&addr).unwrap();

    // This is running on the Tokio runtime, so it will be multi-threaded. The
    // `Arc<Mutex<...>>` allows state to be shared across the threads.
    // let connections = Arc::new(Mutex::new(HashMap::new()));

    let server = listener.incoming().for_each(|conn| {
        // let addr = conn.peer_addr().unwrap();

        // Split the connection into reader and writer
        // Maddeningly, `conn.split` produces `(reader, writer)`
        let (writer, reader) = Framed::new(conn).split();
        let writer = WriteJson::<_, Value>::new(writer);
        let reader = ReadJson::<_, Value>::new(reader);

        // Setup the stop channel
        let (tx, rx) = mpsc::channel();
        let cancel = comm::FutureChannel::new(rx);

        // Setup the communication channel
        let (sink, source) = mpsc::channel::<Value>();
        let source = comm::FutureChannel::new(source);

        // Register the connection
        // connections.lock().unwrap().insert(addr, (tx, sink));

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
