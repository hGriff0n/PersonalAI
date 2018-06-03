extern crate tokio;
extern crate tokio_io;
extern crate tokio_serde_json;
#[macro_use] extern crate serde_json;
extern crate futures;

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

// Get working cross-device communication (move away from home ip)
    // Figure out how to implement discovery so I don't have to hardcode paths
    // Test whether "forwarding" messages works
        // Setup "server state" structures for the handle functions
    // Setup device "name" registration, "to" field, and other setup handling
// Figure out how to use futures 0.2.1
// Once I have this implementation done, develop a python bridge package
// Transition over to getting the modalities to work on the individual channel
// Change the dispatch to a separate app, queried by this
// Develop a tool to automatically launch components/add on the fly
// I'll also work on registering modalities with the python work

// TODO: Figure out how to package this "server" into a single function/class
    // There's a way, I just can't be bothered to fight against the compiler to find it
    // Need to package all of this tokio-wrapper stuff into a common package anyways

fn main() {
    let addr = "127.0.0.1:6142".parse::<SocketAddr>().unwrap();
    let listener = TcpListener::bind(&addr).unwrap();

    // Maintain system-wide connection state
    let connections = Arc::new(Mutex::new(HashMap::new()));

    let server = listener.incoming().for_each(move |conn| {
        // Setup the stop channel
        let (tx, rx) = mpsc::channel();
        let cancel = comm::FutureChannel::new(rx);

        // Setup the communication channel
        let (sink, source) = futures::sync::mpsc::unbounded();

        // Register the connection
        let addr = conn.peer_addr().unwrap();
        {
            connections.lock().unwrap().insert(addr, (tx, sink));
        }

        // Split the connection into reader and writer
        // Maddeningly, `conn.split` produces `(reader, writer)`
        let (writer, reader) = Framed::new(conn).split();
        let writer = WriteJson::<_, Value>::new(writer).sink_map_err(|_| ());
        let reader = ReadJson::<_, Value>::new(reader);

        // Define the reader action
        let read_conns = connections.clone();
        let read_action = reader.for_each(move |msg| {
                handle_request(msg, &read_conns, &addr)
            });

        // Define the writer action
        let write_conns = connections.clone();
        let write_action = source
            .map(move |msg| handle_response(msg, &write_conns, &addr))
            .forward(writer)
            .map(|_| ())
            .map_err(|_| ());

        // Combine the actions into one "packet" for registration with tokio
        let close_conns = connections.clone();
        let action = read_action
            .select2(write_action)
            .select2(cancel)                                // NOTE: This needs to come last in order for it to work
            .map(move |_| {                                 // Remove the connection data from system state
                close_conns.lock().unwrap().remove(&addr);
            })
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

type Connections = Arc<Mutex<HashMap<SocketAddr, (mpsc::Sender<()>, futures::sync::mpsc::UnboundedSender<Value>)>>>;

#[allow(unused_mut)]
fn handle_request(mut msg: Value, conns: &Connections, addr: &SocketAddr) -> Result<(), tokio::io::Error> {
    println!("GOT: {:?}", msg);

    let mut conns = conns.lock().unwrap();
    let iter = conns.iter_mut()
        .filter(|&(&k, _)| k != *addr);

    for (_to, (_, sink)) in iter {
        sink.clone()
            .unbounded_send(json!({ "from": *addr }))
            .expect("Failed to send");
    }

    Ok(())
}

fn handle_response(mut msg: Value, conns: &Connections, addr: &SocketAddr) -> Value {
    msg["resp"] = json!("World");

    if let Some(action) = msg.get("action") {
        if action == "quit" {
            let tx = &conns.lock().unwrap()[addr].0;
            tx.send(()).unwrap();
        }
    }

    msg
}
