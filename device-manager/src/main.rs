extern crate tokio;
extern crate tokio_io;
extern crate tokio_serde_json;
#[macro_use] extern crate serde_json;

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

// Test whether "forwarding" messages works
    // Setup "server state" structures for the handle functions
// Figure out how to make this "service" capable of acting as a client and a server (of forcing a connection to another running instance)
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
        let (sink, source) = mpsc::channel::<Value>();
        let source = comm::FutureChannel::new(source);

        // Register the connection
        let addr = conn.peer_addr().unwrap();
        connections.lock().unwrap().insert(addr, (tx, sink));

        // Split the connection into reader and writer
        // Maddeningly, `conn.split` produces `(reader, writer)`
        let (writer, reader) = Framed::new(conn).split();
        let writer = WriteJson::<_, Value>::new(writer);
        let reader = ReadJson::<_, Value>::new(reader);

        // Define the reader action
        let read_conns = connections.clone();
        let read_action = reader
            .for_each(move |msg| handle_request(msg, &read_conns, &addr));

        // Define the writer action
        let write_conns = connections.clone();

        // This doesn't actually work for sending to a different client (only for echoing)
            // This is only for "sparse" connections (if I'm constantly communicating then some get through)
                // I have to always be sending some messages to the client for some other messages to be sent
            // This actually looks like some sort of deadlock
                // Without the constant communication, something gets stuck and we don't precede
                // But what? None of the source fold stuff gets run after the communication ends
            // The actions are both still active when the halt occurs
            // Changing the sender to a `SyncSender` doesn't solve it either
        // let write_action = source
        //     .transform(move |msg| handle_response(msg, &write_conns, &addr))
        //     .forward(writer);
        let write_action = writer
            .send_all(source
                .transform(move |msg| handle_response(msg, &write_conns, &addr)));

        // Combine the actions into one "packet" for registration with tokio
        let close_conns = connections.clone();
        let action = read_action
            .select2(write_action)
            .select2(cancel)                                // NOTE: This needs to come last in order for it to work
            .map(move |_| {                                 // Remove the connection data from system state
                println!("Closing");
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

type Connections = Arc<Mutex<HashMap<SocketAddr, (mpsc::Sender<()>, mpsc::Sender<Value>)>>>;

#[allow(unused_mut)]
fn handle_request(msg: Value, conns: &Connections, addr: &SocketAddr) -> Result<(), tokio::io::Error> {
    // println!("GOT: {:?}", msg);
    static mut count: i32 = 0;

    // I can get communication to occur if I spawn them fast enough
    // if unsafe { count < 300 } {
    //     unsafe { count += 1; }
    //     conns.lock().unwrap()[addr].1.clone().send(msg).expect("Failed to send");
    //     return Ok(())
    // }

    let mut conns = conns.lock().unwrap();
    // let iter = conns.iter_mut();
        // .filter(|&(&k, _)| k != *addr);
    //     .map(|(_, v)| v);


    // The receiver says it's empty iff I add the address check
    for (to, (_, sink)) in conns.iter() {
        if to != addr
        {
            println!("Sending to {:?}", to);
        // }
            sink.clone()
                .send(json!({ "resp": *addr }))
                .expect("Failed to send");
            // println!("Sent to {:?}", to);
        }
    }

    // conns[addr].1.send(msg).expect("Failed to send message");

    Ok(())
}

fn handle_response(msg: Value, conns: &Connections, addr: &SocketAddr) -> Value {
    // msg["resp"] = json!("World");
    println!("Sending message");

    if let Some(action) = msg.get("action") {
        if action == "quit" {
            let tx = &conns.lock().unwrap()[addr].0;
            tx.send(()).unwrap();
        }
    }

    msg
}
