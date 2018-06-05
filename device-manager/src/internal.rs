
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, mpsc, Mutex};

use futures;
use super::comm;

use tokio;
use tokio::prelude::*;
use tokio::net::{TcpListener, TcpStream};
use tokio_io::codec::length_delimited::Framed;

use serde_json::Value;
use tokio_serde_json::*;

type Signals = (mpsc::Sender<()>, futures::sync::mpsc::UnboundedSender<Value>);

#[derive(Clone)]
pub struct Server {
    conns: Arc<Mutex<HashMap<SocketAddr, Signals>>>,
    mapping: Arc<Mutex<HashMap<String, SocketAddr>>>,
}

impl Server {
    // Interface methods (ie. customization points)
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


    // Automatic methods
    pub fn new() -> Self {
        Self{
            conns: Arc::new(Mutex::new(HashMap::new())),
            mapping: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn spawn(self, listen_addr: SocketAddr, parent_addr: Option<SocketAddr>) {
        // Construct the server action
        #[allow(unused_mut)]
        let mut server = TcpListener::bind(&listen_addr)
            .unwrap()
            .incoming();

        // TODO: Merge it into the server stream
            // I think I want to create a separate "client" action to handle some stuff
        if let Some(addr) = parent_addr {
            let _client_stream = TcpStream::connect(&addr);
            // record the server address
            // server = server.
        }

        // Complete the server action
        let server = server
            .for_each(move |conn| {
                // Setup the stop channel
                let (tx, cancel) = mpsc::channel();
                let cancel = comm::FutureChannel::new(cancel);

                // Setup the communication channel
                let (sink, source) = futures::sync::mpsc::unbounded();

                // Register the connection
                let addr = conn.peer_addr().unwrap();
                self.add_connection(addr, (tx, sink));

                // Split the connection into reader and writer
                // Maddeningly, `conn.split` produces `(reader, writer)`
                let (writer, reader) = Framed::new(conn).split();
                let writer = WriteJson::<_, Value>::new(writer).sink_map_err(|_| ());
                let reader = ReadJson::<_, Value>::new(reader);

                // Define the reader action
                let read_state = self.clone();
                let read_action = reader.for_each(move |msg| read_state.handle_request(msg, &addr));

                // Define the writer action
                let write_state = self.clone();
                let write_action = source
                    .map(move |msg| write_state.handle_response(msg, &addr))
                    .forward(writer)
                    .map(|_| ())
                    .map_err(|_| ());

                // Combine the actions into one "packet" for registration with tokio
                let close_state = self.clone();
                let action = read_action
                    .select2(write_action)
                    .select2(cancel)
                    .map(move |_| close_state.drop_connection(addr))
                    .map_err(|_| ());

                // Finally spawn the connection
                tokio::spawn(action);
                Ok(())
            })
            .map_err(|err| println!("Server error: {:?}", err));

        /*
        if let Some(addr) = parent_addr {
            let client = TcpStream::connect(&addr)
                .and_then(move |conn| {

                })
                .map_err(|err| {
                    println!("Client error: {:?}", err)
                });

            let action = server
                .select2(client)
                .map_err(|_| ());

            tokio::run(action);

        } else {
            tokio::run(server);
        }
        */

        // Start the server and tokio runtime
        tokio::run(server);
    }
}
