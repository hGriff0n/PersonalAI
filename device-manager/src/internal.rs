
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

// TODO: Improve the request/response handling
// TODO: Implement actual handshake negotiation

#[derive(Clone)]
pub struct Server {
    conns: Arc<Mutex<HashMap<SocketAddr, Signals>>>,
    mapping: Arc<Mutex<HashMap<String, SocketAddr>>>,
    parent_addr: Option<SocketAddr>,
}

impl Server {
    pub fn new(parent_addr: Option<SocketAddr>) -> Self {
        Self{
            conns: Arc::new(Mutex::new(HashMap::new())),
            mapping: Arc::new(Mutex::new(HashMap::new())),
            parent_addr: parent_addr
        }
    }

    // Interface methods (ie. customization points)
    fn handle_request(&mut self, mut msg: Value, addr: &SocketAddr) -> Result<(), tokio::io::Error> {
        info!("Got {:?} from {:?}", msg, addr);

        match msg.get("action").and_then(|act| act.as_str()) {
            Some("handshake") => {
                let mut roles = self.mapping.lock().unwrap();
                let role = msg.get("hooks").unwrap()[0].as_str().unwrap();

                info!("Adding role {} for IP {:?}", role, *addr);
                roles.insert(role.to_string(), *addr);
                return Ok(());
            },
            Some("quit") => {
                let mut conns = self.conns.lock().unwrap();
                conns[&addr].0.send(()).expect("Failed to close connection");
                // return Ok(());
            },
            // Some("copy") => {
            //     tasks::CopyAction::new(self, addr).spawn(msg);
            // }
            None => {
                return Ok(());
            },
            _ => ()
        }

        let sender_addr = msg["from"].as_str().and_then(|addr| addr.parse::<SocketAddr>().ok());
        msg["from"] = json!(*addr);

        // TODO: Remove duplication of the code base
        let dest_opt = msg.get("routing").and_then(|dst| dst.as_str()).and_then(|dest| Some(dest.to_string()));
        if let Some(dest) = dest_opt {
            if let Some(dest) = self.mapping.lock().unwrap().get(&dest) {

                // Send the ACK message to the creating app
                if let Some(sender) = sender_addr {
                    if *dest != sender {
                        let mut new_msg = json!({ "from": sender, "routing": "sender" , "action": "ack" });
                        new_msg["text"] = msg["text"].take();
                        new_msg["stop"] = msg["stop"].clone();

                        let (_, ref sink) = self.conns.lock().unwrap()[&sender];

                        info!("Acking {:?} to {:?}", new_msg, sender);
                        sink.clone()
                            .unbounded_send(new_msg)
                            .expect("Failed to send");
                    }
                }

                let (_, ref sink) = self.conns.lock().unwrap()[dest];

                info!("Sending {:?} to {:?}", msg, dest);
                sink.clone()
                    .unbounded_send(msg.clone())
                    .expect("Failed to send");

            } else if dest == "sender" {
                let (_, ref sink) = self.conns.lock().unwrap()[addr];

                info!("Responding {:?} to {:?}", msg, addr);
                sink.clone()
                    .unbounded_send(msg.clone())
                    .expect("Failed to send");

            } else if dest == "broadcast" {
                let conns = self.conns.lock().unwrap();
                let iter = conns.iter();

                for (&dest, (_, sink)) in iter {
                    if dest != *addr {
                        info!("Broadcasting {:?} to {:?}", msg, dest);
                        sink.clone()
                            .unbounded_send(msg.clone())
                            .expect("Failed to send");
                    }
                }
            }
        }

        Ok(())
    }

    #[allow(unused_variables)]
    fn handle_server_request(&self, msg: Value) -> Result<(), tokio::io::Error> {
        Ok(())
    }

    #[allow(unused_variables, unused_mut)]
    fn handle_response(&self, mut msg: Value, addr: &SocketAddr) -> Value {
        // msg["resp"] = json!("World");

        // if !msg.get("was_handshake").unwrap().as_bool().unwrap() {
        //     msg["play"] = json!("Aerosmith");
        // }

        msg
    }

    #[allow(unused_mut)]
    fn handle_server_response(&self, mut msg: Value) -> Value {
        msg
    }

    fn add_connection(&self, addr: SocketAddr, signals: Signals) {
        self.conns.lock().unwrap().insert(addr, signals);
    }

    fn drop_connection(&mut self, addr: SocketAddr) {
        self.conns.lock().unwrap().remove(&addr);
    }

    fn shutdown(self) {
        for (_, (close, _)) in self.conns.lock().unwrap().iter() {
            close.clone().send(()).unwrap();
        }
    }
}


// Spawn all the tokio actions necessary to run the described server
pub fn spawn(server: Server, listen_addr: SocketAddr) {
    let client = server.clone();
    let parent = server;

    // Construct the server action
    #[allow(unused_mut)]
    let mut server = TcpListener::bind(&listen_addr)
        .unwrap()
        .incoming();

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
            info!("New connection: {}", addr);
            parent.add_connection(addr, (tx, sink));

            // Split the connection into reader and writer
            // Maddeningly, `conn.split` produces `(reader, writer)`
            let (writer, reader) = Framed::new(conn).split();
            let writer = WriteJson::<_, Value>::new(writer).sink_map_err(|err| { error!("Sink Error: {:?}", err); });
            let reader = ReadJson::<_, Value>::new(reader);

            // Define the reader action
            let mut read_state = parent.clone();
            let read_action = reader
                .for_each(move |msg| read_state.handle_request(msg, &addr))
                .map_err(|err| { error!("Read Error: {:?}", err); });

            // Define the writer action
            #[allow(unused_mut)]
            let mut write_state = parent.clone();
            let write_action = source
                .map(move |msg| write_state.handle_response(msg, &addr))
                .forward(writer)
                .map(|_| ())
                .map_err(|err| { error!("Write Error: {:?}", err); });

            // Combine the actions into one "packet" for registration with tokio
            let mut close_state = parent.clone();
            let action = read_action
                .select2(write_action)
                .select2(cancel)
                .map(move |_| close_state.drop_connection(addr))
                .map_err(|_| ());

            // Finally spawn the connection
            tokio::spawn(action);
            Ok(())
        })
        .map_err(|err| error!("Server Error: {:?}", err));

    if let Some(paddr) = client.parent_addr {
        let client_conn = TcpStream::connect(&paddr)
            .and_then(move |conn| {
                    // Split the connection into reader and writer
                let (writer, reader) = Framed::new(conn).split();
                let writer = WriteJson::<_, Value>::new(writer).sink_map_err(|_| ());
                let reader = ReadJson::<_, Value>::new(reader);

                // Setup the stop channel
                let (tx, cancel) = mpsc::channel();
                let cancel = comm::FutureChannel::new(cancel);

                // Setup the communication channel
                let (sink, source) = futures::sync::mpsc::unbounded::<Value>();
                client.add_connection(paddr, (tx, sink.clone()));

                // Unilaterally send a message to the server
                sink.unbounded_send(json!({ "action": "register" })).unwrap();

                // Define the reader action
                #[allow(unused_mut)]
                let mut read_state = client.clone();
                let read_action = reader
                    .for_each(move |msg| read_state.handle_server_request(msg));

                // Define the writer action
                #[allow(unused_mut)]
                let mut write_state = client.clone();
                let write_action = source
                    .map(move |msg| write_state.handle_server_response(msg))
                    .forward(writer)
                    .map(|_| ())
                    .map_err(|_| ());

                // Assemble the actions into a single "tokio" packet
                let action = read_action
                    .select2(write_action)
                    .select2(cancel)                                // NOTE: This needs to come last in order for it to work
                    .map(move |_| client.shutdown())
                    .map_err(|_| ());                               // NOTE: I'm ignoring all errors for now

                // Finally spawn the connection
                tokio::spawn(action);
                Ok(())
            })
            .map_err(|err| {
                error!("Client error: {:?}", err)
            });

        let action = server
            .join(client_conn)
            .map(|_| ())
            .map_err(|_| ());

        tokio::run(action);

    } else {
        tokio::run(server);
    }
}
