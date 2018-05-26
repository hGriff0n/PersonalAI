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

// For the moment, I'll just pass simple json messages back and forth
// https://github.com/carllerche/tokio-serde-json

use tokio::prelude::*;
use tokio::net::TcpListener;
// use tokio_io::AsyncRead;
// use tokio_io::codec::LinesCodec;
use tokio_io::codec::length_delimited;

use std::net::SocketAddr;

use serde_json::Value;
use tokio_serde_json::*;

// Extend the example to produce an echo server
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
        // Delimit frames using a length header
        let framed = length_delimited::FramedRead::new(conn);

        // Deserialize frames
        let deserialized = ReadJson::<_, Value>::new(framed)
            .map_err(|e| println!("ERR: {:?}", e));

        // Spawn a task that prints all received messages to STDOUT
        tokio::spawn(deserialized.for_each(|msg| {
            println!("GOT: {:?}", msg);
            Ok(())
        }));

        Ok(())
    })

    // let server = listener.incoming().for_each(|tcp| {
    //     // Split up the read and write halves
    //     let (writer, reader) = tcp.framed(LinesCodec::new()).split();
    //     let (tx, rx) = std::sync::mpsc::channel();
    //     let cancel = comm::Canceller{ rx: rx };

    //     // Perform server duties
    //     let action = reader.map(move |line| {
    //             tx.send(()).unwrap();               // Signal the canceller to complete
    //             line
    //         })
    //         .forward(writer)                        // Forward the data onto the client
    //         .select2(cancel)                        // Allow the cancel signal to stop execution
    //         .map(|_| {})
    //         .map_err(|err| println!("error"));

    //     tokio::spawn(action);
    //     Ok(())
    // })
    .map_err(|err| {
        println!("Server error: {:?}", err)
    });

    // Start the server and tokio runtime
    tokio::run(server);
}
