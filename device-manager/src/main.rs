extern crate tokio;
extern crate tokio_io;
extern crate tokio_serde_json;
#[macro_use] extern crate serde_json;
extern crate futures;

mod internal;
mod comm;

// This program acts as the interaction manager for the individual device,
// Collecting and dispatching requests to the global server from modalities
// While maintaining and handling system level state/operations

use std::env;
use std::net::SocketAddr;

// Figure out how to use futures 0.2.1 within this code
// Improve this code to production quality
    // Handle/log errors
    // Improve the config file situation
        // Figure out how to split config files (to allow for spawning device-manager/system-manager from the same codebase)
    // Improve the process of abstracting server development
        // In case I want to be able to provide different server impls
            // I know how to do it, can't get the compiler to agree
// Get working cross-device communication (move away from home ip)
    // Figure out how to handle registration/setup for modalities
    // Modify dispatch to not use hardcoded logic, instead use associated keys/etc.
// I'll also work on registering modalities with the python work

fn main() {
    // TODO: Grab server data from the command line parameters (using clap)

    // Setup initial listener state
    let addr = "127.0.0.1:6142".parse::<SocketAddr>().unwrap();
    let parent = None;

    // Create the server
    let server = internal::Server::new(parent);

    // TODO: Spawn any persistent system tools and register them with the server
        // Non-persistent tasks can be spawned by the server as needed (using tokio)

    // Spawn up the server
    internal::spawn(server, addr);
}

// API Documentation:
//  tokio-serde-json: https://github.com/carllerche/tokio-serde-json
