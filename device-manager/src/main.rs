
extern crate chrono;
extern crate clap;
extern crate fern;
extern crate futures;
extern crate get_if_addrs;
#[macro_use]
extern crate log;
extern crate multimap;
extern crate serde;
#[macro_use]
extern crate serde_derive;
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
mod message;

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
    // The best option would be to transition primary config to utilize config-rs
    // We then utilize 'clap' to overwrite argument values (and locate the config file)
        // NOTE: We have to implement config-rs::Source for 'clap' to enable this

// fn main() -> Result<(), Error> {
fn main() {
    let args = load_configuration();
    logging::launch(&args).expect("Failed to initialize logging");

    // Construct the indexer
    // TODO: Loading the index from file causes some start-up delay
    let (index, writer) = seshat::index::Index::new();
    trace!("Created device fs index");

    // Create the device manager
    let (tx, cancel) = mpsc::channel();
    let manager = device::DeviceManager::new(index, tx.clone());
    trace!("Created device state manager");

    // Create the seshat indexer (and search engine portal)
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
        .select2(indexer)
        ;

    // Add in the ability to pre-emptively short-circuit all computations
    let device = device
        .select2(networking::comm::FutureChannel::new(cancel))
        .map(move |_| { trace!("Closing device") })
        .map_err(move |_| {});
    trace!("Created tokio task description");

    // Spawn the futures in the tokio event loop
    info!("Launching tokio task chain");
    tokio::run(device);
    info!("System shutdown");
}


// Parse the command line arguments
fn load_configuration<'a>() -> clap::ArgMatches<'a> {
    // use clap::Arg;

    // Create the base app and arguments
    let app = clap::App::new("Device Manager")
        .version("0.1")
        .author("Grayson Hooper <ghooper96@gmail.com>")
        .about("Manages device state and communication");

    // Add arguments for other system aspects
    let app = indexer::add_args(app);
    let app = server::add_args(app);
    let app = logging::add_args(app);

    // Return the command line matches
    app.get_matches()
}
