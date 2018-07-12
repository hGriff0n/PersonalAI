
use std::net::SocketAddr;
use std::sync::mpsc;

use tokio;
use tokio::prelude::*;
use tokio::net::TcpListener;

use server::comm;
use server::spawn::spawn_connection;

use super::device::AiManager;

pub fn serve(addr: SocketAddr) {
    // Setup stop communication
    let (tx, cancel) = mpsc::channel();
    let cancel = comm::FutureChannel::new(cancel);

    // Create device manager state
    let manager = AiManager::new(tx.clone());


    // Create the listening server
    let server = TcpListener::bind(&addr)
        .unwrap()
        .incoming()
        .for_each(move |conn| Ok(spawn_connection(conn, manager.clone())))
        .map_err(|err| error!("Server Error: {:?}", err));

    // and spawn it
    let device = server
        .select2(cancel)
        .map(|_| { trace!("Closing device") })
        .map_err(|_err| { error!("Closing error") });

    tokio::run(device);

    info!("System Shutdown");
}
