
use std::net::SocketAddr;
use std::sync::mpsc;

use std::io;
use std::result::Result;

use tokio;
use tokio::prelude::*;
use tokio::net::{ TcpListener, TcpStream };

use futures;

use server::comm;
use server::spawn::spawn_connection;

use super::device::DeviceManager;

// TODO: What types do I need to unify this
pub fn create_server(device: DeviceManager, addr: SocketAddr, parent: Option<SocketAddr>) -> Box<dyn futures::Future<Item=(), Error=()> + Send> {
    let ai_device = device.clone();
    let server = TcpListener::bind(&addr)
        .unwrap()
        .incoming()
        .for_each(move |conn| Ok(spawn_connection(conn, device.clone())))
        .map_err(|err| error!("Server Error: {:?}", err));

    if let Some(paddr) = parent {
        info!("Initializing web-node device-manager");
        info!("Connecting to parent device at {}", paddr);

        let client = TcpStream::connect(&paddr)
            .and_then(move |conn| Ok(spawn_connection(conn, ai_device)))
            .map_err(|err| { error!("Client error: {:?}", err) });

        Box::new(server.join(client).map(|_| ()))

    } else {
        info!("Initializing standalone device-manager");
        Box::new(server)
    }
}

// TODO: This should allow us to shut-down the server from within, but it doesn't. Why?
pub fn serve(addr: SocketAddr, parent: Option<SocketAddr>) {
    // NOTE: Everything in this function will likely be moved to some different module (or even main)
    // This allows us to more generally bring in new capabilities/etc. as we no longer requre the server code to launch tokio
    // Setup stop communication
    let (tx, cancel) = mpsc::channel();
    let cancel = comm::FutureChannel::new(cancel);

    let manager = DeviceManager::new(parent, tx.clone());

    // TODO: Spawn any timer threads (eg. Seshat indexing)
    // NOTE: We will likely move this to a different file (or even main)

    let server = create_server(manager.clone(), addr, parent);

    // Setup the cancellation
    let device = server
        .select2(cancel)
        .map(move |_| { trace!("Closing device") })
        .map_err(|_| { error!("Error during device closing") });

    // Run the futures within tokio
    tokio::run(device);
    info!("System shutdown");
}
