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
    // Test whether "forwarding" messages works
// Figure out how to use futures 0.2.1
// Once I have this implementation done, develop a python bridge package
// Transition over to getting the modalities to work on the individual channel
    // Figure out how to handle registration/setup
    // Generalize this code to enable server-server-client hierarchy
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
    let state = ServerState::new();

    let server = listener.incoming().for_each(move |conn| {
        // Setup the stop channel
        let (tx, cancel) = mpsc::channel();
        let cancel = comm::FutureChannel::new(cancel);

        // Setup the communication channel
        let (sink, source) = futures::sync::mpsc::unbounded();

        // Register the connection
        let addr = conn.peer_addr().unwrap();
        state.add_connection(addr, (tx, sink));

        // Split the connection into reader and writer
        // Maddeningly, `conn.split` produces `(reader, writer)`
        let (writer, reader) = Framed::new(conn).split();
        let writer = WriteJson::<_, Value>::new(writer).sink_map_err(|_| ());
        let reader = ReadJson::<_, Value>::new(reader);

        // Define the reader action
        let read_state = state.clone();
        let read_action = reader.for_each(move |msg| read_state.handle_request(msg, &addr));

        // Define the writer action
        let write_state = state.clone();
        let write_action = source
            .map(move |msg| write_state.handle_response(msg, &addr))
            .forward(writer)
            .map(|_| ())
            .map_err(|_| ());

        // Combine the actions into one "packet" for registration with tokio
        let close_state = state.clone();
        let action = read_action
            .select2(write_action)
            .select2(cancel)                                // NOTE: This needs to come last in order for it to work
            .map(move |_| close_state.drop_connection(addr))
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


type Signals = (mpsc::Sender<()>, futures::sync::mpsc::UnboundedSender<Value>);

// Holds all information for managing the server's state across all connections
// Also acts as a customization point for the server's behavior
    // I think I need to add another point for handling registration communications (and enable middleware)
#[derive(Clone)]
struct ServerState {
    conns: Arc<Mutex<HashMap<SocketAddr, Signals>>>,
    mapping: Arc<Mutex<HashMap<String, SocketAddr>>>,
}

impl ServerState {
    pub fn new() -> Self {
        Self{
            conns: Arc::new(Mutex::new(HashMap::new())),
            mapping: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn handle_request(&self, msg: Value, addr: &SocketAddr) -> Result<(), tokio::io::Error> {
        println!("GOT: {:?}", msg);

        #[allow(unused_mut)]
        let mut new_msg = json!({ "from": *addr });

        if let Some(addr) = msg.get("to") {
            if let Some(addr) = addr.as_str() {
                if let Ok(addr) = addr.parse::<SocketAddr>() {
                    let mut conns = self.conns.lock().unwrap();
                    conns[&addr].1.clone().unbounded_send(new_msg).expect("Failed to send");
                }
            }

        } else {
            let mut conns = self.conns.lock().unwrap();
            let iter = conns.iter_mut()
                .filter(|&(&k, _)| k != *addr);

            for (_to, (_, sink)) in iter {
                sink.clone()
                    .unbounded_send(json!({ "from": *addr }))
                    .expect("Failed to send");
            }
        }

        Ok(())
    }

    pub fn handle_response(&self, mut msg: Value, addr: &SocketAddr) -> Value {
        msg["resp"] = json!("World");

        if let Some(action) = msg.get("action") {
            if action == "quit" {
                let tx = &self.conns.lock().unwrap()[addr].0;
                tx.send(()).unwrap();
            }
        }

        msg
    }

    pub fn add_connection(&self, addr: SocketAddr, signals: Signals) {
        self.conns.lock().unwrap().insert(addr, signals);
    }

    pub fn drop_connection(&self, addr: SocketAddr) {
        self.conns.lock().unwrap().remove(&addr);
    }
}
