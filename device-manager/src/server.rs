
use std::net::SocketAddr;

use clap;
use futures;
use tokio::prelude::*;
use tokio::net::{ TcpListener, TcpStream };

use networking::spawn::spawn_connection;
use super::device::DeviceManager;

// NOTE: I need the 'Box' type because I'm returning 2 different 'futures::Future' types
// The `impl Trait` syntax doesn't work in this case because of compiler type-checking requirements
fn create_server(device: DeviceManager, addr: SocketAddr, parent: Option<SocketAddr>) -> Box<dyn futures::Future<Item=(), Error=()> + Send> {
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

pub fn launch<'a>(device: DeviceManager, args: &'a clap::ArgMatches) -> impl futures::Future {
    // Parse out the connection addresses
    let addr = args.value_of("addr")
        .unwrap_or("127.0.0.1:6142")
        .parse::<SocketAddr>()
        .unwrap();
    trace!("Parsed device-server listening address: {:?}", addr);

    // let parent = "127.0.0.1:6141".parse::<SocketAddr>().ok();
    let parent = None;
    trace!("Parsed device-server parent address: {:?}", parent);

    // Create the server "futures"
    create_server(device.clone(), addr, parent)
}

pub fn add_args<'a, 'b>(app: clap::App<'a, 'b>) -> clap::App<'a, 'b> {
    use clap::Arg;

    app.arg(Arg::with_name("addr")
            .long("addr")
            .value_name("IP")
            .help("Listening port and address for the device manager")
            .takes_value(true))
}
