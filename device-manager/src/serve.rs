
use std::net::SocketAddr;
use std::sync::mpsc;

use tokio;
use tokio::prelude::*;
use tokio::net::{ TcpListener, TcpStream };

use server::comm;
use server::spawn::spawn_connection;

use super::device::DeviceManager;


// TODO: This should allow us to shut-down the server from within, but it doesn't. Why?
pub fn serve(addr: SocketAddr, parent: Option<SocketAddr>) {
    // Setup stop communication
    let (tx, cancel) = mpsc::channel();
    let cancel = comm::FutureChannel::new(cancel);

    // Create manager
    let manager = DeviceManager::new(parent, tx.clone());
    let ai_client = manager.clone();


    // Spawn the listening server
    let server = TcpListener::bind(&addr)
        .unwrap()
        .incoming()
        .for_each(move |conn| Ok(spawn_connection(conn, manager.clone())))
        .map_err(|err| error!("Server Error: {:?}", err));


    // If there is an ai server running (ie. a "parent server") connect to it
    if let Some(paddr) = parent {
        let client = TcpStream::connect(&paddr)
            .and_then(move |conn| Ok(spawn_connection(conn, ai_client)))
            .map_err(|err| { error!("Client error: {:?}", err) });

        let device = server
            .join(client)
            .select2(cancel)
            .map(move |_| { trace!("Closing device") })
            .map_err(|_err| { error!("Closing error") });

        tokio::run(device);

    // Otherwise, just start listening
    } else {
        let device = server
            .select2(cancel)
            .map(|_| { trace!("Closing device") })
            .map_err(|_err| { error!("Closing error") });

        tokio::run(device);
    }

    info!("System Shutdown");
}
