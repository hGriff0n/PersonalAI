
use std::net::SocketAddr;
use std::sync::mpsc;

use tokio;
use tokio::prelude::*;
use tokio::net::TcpListener;

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

    let server = TcpListener::bind(&addr)
        .unwrap()
        .incoming()
        .for_each(move |conn| Ok(spawn_connection(conn, manager.clone())))
        .map_err(|err| error!("Server Error: {:?}", err));

    if let Some(_paddr) = parent {
        // Setup stop communication
        // let (ntx, ncancel) = mpsc::channel();
        // let ncancel = comm::FutureChannel::new(ncancel);

        // let client_tx = ntx.clone();
        // #[allow(unused_must_use)]
        // let server = server
        //     .select2(cancel)
        //     .map(move |_| { client_tx.send(()); })
        //     .map_err(|_| ());

        // // Create ai client
        // let negotiator = AiClient::new(ntx.clone());

        // let client = TcpStream::connect(&paddr)
        //     .and_then(move |conn| Ok(spawn_connection(conn, negotiator.clone())))
        //     .map_err(|err| { error!("Client error: {:?}", err) });

        // let server_tx = tx.clone();
        // #[allow(unused_must_use)]
        // let device = server
        //     .join(client)
        //     .select2(ncancel)
        //     .map(move |_| { server_tx.send(()); trace!("Closing device") })
        //     .map_err(|_err| { error!("Closing error") });

        // tokio::run(device);

    } else {
        let device = server
            .select2(cancel)
            .map(|_| { trace!("Closing device") })
            .map_err(|_err| { error!("Closing error") });

        tokio::run(device);
    }

    info!("System Shutdown");
}
