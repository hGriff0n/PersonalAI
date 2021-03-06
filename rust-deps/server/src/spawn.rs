
use std::sync::mpsc;

use tokio;
use tokio::prelude::*;
use tokio::net::TcpStream;
#[allow(deprecated)]
use tokio_io::codec::length_delimited;
// use tokio::codec::*;

use futures;
use serde_json::Value;
use tokio_serde_json::*;

use super::*;
use super::comm;


pub fn spawn_connection<Server: 'static + BasicServer>(conn: TcpStream, server: Server) {
    // Setup stop communication
    let (tx, cancel) = mpsc::channel();
    let cancel = comm::FutureChannel::new(cancel);

    // Setup communication channels
    let (sink, source) = futures::sync::mpsc::unbounded();

    // Register the connection
    let addr = conn.peer_addr().expect("Failed to extract peer address from TcpStream");
    server.add_connection(addr, tx, sink).expect("Failed to add connection");

    // Setup the json communicators
    // TODO: We need to update this has the length_delimited stuff has been deprecated and moved
    // Unfortunately, the WriteJson repo relies on the writer to have a `BytesMut` sink while the updated codec has a `Bytes` sink
    // This requirement is derived from tokio_serde
    // let (writer, reader) = Framed::new(conn, length_delimited::LengthDelimitedCodec::new()).split();
    // let writer = WriteJson::<_, Value>::new(writer);

    let (writer, reader) = length_delimited::Framed::new(conn).split();
    let writer = WriteJson::<_, Value>::new(writer)
        .sink_map_err(|err| { error!("Error in json serialization: {:?}", err); });
    let reader = ReadJson::<_, Value>::new(reader);

    // Define the handle for incoming communication
    let mut read_state = server.clone();
    let read_action = reader
        .for_each(move |msg| read_state.handle_request(msg, &addr))
        .map(|_| ())
        .map_err(|err| { error!("Socket read error: {:?}", err); });

    // Define the handle for outgoing communication
    let mut write_state = server.clone();
    let write_action = source
        .map(move |msg| write_state.handle_response(msg, &addr))
        .forward(writer)
        .map(|_| ())
        .map_err(|err| { error!("Socket write error: {:?}", err); });

    // Combine the actions for tokio registration
    let mut close_state = server.clone();
    let action = read_action
        .select2(write_action)
        .select2(cancel)
        .map(move |_| close_state.drop_connection(addr))
        .map_err(|_| { error!("Error closing the server"); });

    // Spawn the connection
    tokio::spawn(action);
}
