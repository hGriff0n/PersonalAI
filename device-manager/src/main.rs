
extern crate chrono;
extern crate clap;
extern crate fern;
extern crate futures;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_json;
extern crate tokio;
extern crate walkdir;

// Local crates
extern crate server as networking;
extern crate seshat;
extern crate tags;

// Local modules
mod device;
mod indexer;
mod logging;
mod server;

// Imports
use std::sync::mpsc;

use futures::Future;

/*
This program acts primarily as an interaction manager for the individual device that operates as
Part of the network, that enables this ai to function properly. It handles the collection and
Dispatch of all requests and responses to/from the global network by the individual modalities
That are slated to run on the device. It is also responsible for maintaining and handling system-level
State and operations, particularly the indexing of the local file system and the measurement
Of local device statistics, among others.
*/

// TODO: Possibly add in config file support
    // https://github.com/casey/clap-config
    // TODO: Might convert entirely to config file (config-rs)

fn main() {
    let args = parse_command_line();

    logging::launch(&args).expect("Failed to initialize logging");
    trace!("Logger setup properly");

    // Construct the indexer
    let (index, writer) = match args.value_of("index-cache") {
        Some(file) => {
            let file = std::path::Path::new(file);
            info!("Loading index from {:?}", file);
            seshat::index::Index::from_file(&file)
        },
        None => seshat::index::Index::new()
    };
    trace!("Created device fs index");

    // Create the device manager
    let (tx, cancel) = mpsc::channel();
    let manager = device::DeviceManager::new(index, tx.clone());
    trace!("Created device state manager");

    let indexer = indexer::launch(manager.clone(), &args, writer);
    trace!("Created async fs indexer");

    // TODO: Figure out how these will interact with the new system
    // TODO: Spawn any persistent system tools and register them with the server
        // Non-persistent tasks can be spawned by the server as needed (using tokio)

    // trace!("Spawned persistant tasks");

    let server = server::launch(manager.clone(), &args);
    trace!("Created async server");

    // Combine all futures
    let device = server
        .select2(indexer);

    // Add in the ability to pre-emptively short-circuit all computations
    let device = device
        .select2(networking::comm::FutureChannel::new(cancel))
        .map(move |_| { trace!("Closing device") })
        .map_err(move |_| {});
    trace!("Created tokio task description");

    // Spawn the futures in the tokio event loop
    tokio::run(device);
    info!("System shutdown");
}


// Parse the command line arguments
fn parse_command_line<'a>() -> clap::ArgMatches<'a> {
    use clap::Arg;

    let app = clap::App::new("Device Manager")
        .version("0.1")
        .author("Grayson Hooper <ghooper96@gmail.com>")
        .about("Manages device state and communication")
        .arg(Arg::with_name("index-cache")
            .long("index-cache")
            .help("location of the index cache storage file")
            .value_name("JSON")
            .takes_value(true));

    // Add arguments for other system aspects
    let app = indexer::add_args(app);
    let app = server::add_args(app);
    let app = logging::add_args(app);

    // Return the command line matches
    app.get_matches()
}

// API Documentation:
//  tokio: https://github.com/tokio-rs/tokio
//  tokio-serde-json: https://github.com/carllerche/tokio-serde-json
//  clap: https://github.com/kbknapp/clap-rs
