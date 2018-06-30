
use std::net::SocketAddr;
use std::sync::mpsc;

use tokio;
use tokio::prelude::*;
use tokio::net::{ TcpListener, TcpStream };
use tokio_io::codec::length_delimited;

use futures;
use serde_json::Value;
use tokio_serde_json::*;

use super::servers;
use super::servers::BasicServer;
use super::comm;


fn spawn_connection<Server: 'static + BasicServer>(conn: TcpStream, server: Server) {
    // Setup stop communication
    let (tx, cancel) = mpsc::channel();
    let cancel = comm::FutureChannel::new(cancel);

    // Setup communication channels
    let (sink, source) = futures::sync::mpsc::unbounded();

    // Register the connection
    let addr = conn.peer_addr().unwrap();
    server.add_connection(addr, tx, sink)
          .expect("Failed to add connection");

    // Setup the json communicators
    let (writer, reader) = length_delimited::Framed::new(conn).split();
    let writer = WriteJson::<_, Value>::new(writer)
        .sink_map_err(|err| { error!("WriteJson: {:?}", err); });
    let reader = ReadJson::<_, Value>::new(reader);

    // Define the handle for incoming communication
    let mut read_state = server.clone();
    let read_action = reader
        .for_each(move |msg| read_state.handle_request(msg, &addr))
        .map(|_| ())
        .map_err(|err| { error!("Read Error: {:?}", err); });

    // Define the handle for outgoing communication
    let mut write_state = server.clone();
    let write_action = source
        .map(move |msg| write_state.handle_response(msg, &addr))
        .forward(writer)
        .map(|_| ())
        .map_err(|err| { error!("Write Error: {:?}", err); });

    // Combine the actions for tokio registration
    let close_state = server.clone();
    let action = read_action
        .select2(write_action)
        .select2(cancel)
        .map(move |_| close_state.drop_connection(addr))
        // .map_err(|err| { error!("Closing Error: {:?}", err); });
        .map_err(|_| { error!("Closing error"); });

    // Spawn the connection
    tokio::spawn(action);
}


// TODO: This should allow us to shut-down the server from within, but it doesn't. Why?
pub fn serve(addr: SocketAddr, parent: Option<SocketAddr>) {
    // Setup stop communication
    let (tx, cancel) = mpsc::channel();
    let cancel = comm::FutureChannel::new(cancel);

    // Create manager
    let manager = servers::DeviceManager::new(parent, tx.clone());

    let server = TcpListener::bind(&addr)
        .unwrap()
        .incoming()
        .for_each(move |conn| Ok(spawn_connection(conn, manager.clone())))
        .map_err(|err| error!("Server Error: {:?}", err));

    if let Some(paddr) = parent {
        // Setup stop communication
        let (ntx, ncancel) = mpsc::channel();
        let ncancel = comm::FutureChannel::new(ncancel);

        let client_tx = ntx.clone();
        #[allow(unused_must_use)]
        let server = server
            .select2(cancel)
            .map(move |_| { client_tx.send(()); })
            .map_err(|_| ());

        // Create ai client
        let negotiator = servers::AiClient::new(ntx.clone());

        let client = TcpStream::connect(&paddr)
            .and_then(move |conn| Ok(spawn_connection(conn, negotiator.clone())))
            .map_err(|err| { error!("Client error: {:?}", err) });

        let server_tx = tx.clone();
        #[allow(unused_must_use)]
        let device = server
            .join(client)
            .select2(ncancel)
            .map(move |_| { server_tx.send(()); trace!("Closing device") })
            .map_err(|_err| { error!("Closing error") });

        tokio::run(device);

    } else {
        let device = server
            .select2(cancel)
            .map(|_| { trace!("Closing device") })
            .map_err(|_err| { error!("Closing error") });

        tokio::run(device);
    }

    info!("System Shutdown");
}
