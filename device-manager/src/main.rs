extern crate tokio;
extern crate tokio_io;
extern crate tokio_serde_json;
#[macro_use] extern crate serde_json;
extern crate futures;

extern crate config;
// NOTE: This won't be used until this project switches to a more development focus
extern crate app_dirs;

mod internal;
mod comm;

// This program acts as the interaction manager for the individual device,
// Collecting and dispatching requests to the global server from modalities
// While maintaining and handling system level state/operations

use std::net::SocketAddr;
use std::collections::HashMap;

// Get working cross-device communication (move away from home ip)
    // Test whether "forwarding" messages works
// Transition over to getting the modalities to work on the individual channel
    // Figure out how to handle registration/setup
    // Generalize this code to enable server-server-client hierarchy
// Figure out how to use futures 0.2.1 within this code
// Once I have this implementation done, develop a python bridge package
// Improve this code to production quality
    // Handle/log errors
    // Improve the config file situation
// Change the dispatch to a separate app, queried by this
// Develop a tool to automatically launch components/add on the fly
// I'll also work on registering modalities with the python work

// TODO: Figure out how to package this "server" into a single function/class
    // There's a way, I just can't be bothered to fight against the compiler to find it
    // Need to package all of this tokio-wrapper stuff into a common package anyways

fn main() {
    // Grab device manager config data
    // let config_dir = app_dirs::app_root(app_dirs::AppDataType::UserConfig, &APP_INFO)
    //     .expect("Couldn't create user config directory");

    // Example of how to use the config crate
    let mut settings = config::Config::default();
    settings
        .merge(vec![config::File::with_name("conf/conf.yaml")])
        .unwrap();
    let settings = settings.try_into::<HashMap<String, String>>().unwrap();

    // Setup initial listener state
    let addr = settings
        .get("addr")
        .unwrap_or(&"127.0.0.1:6142".to_string())
        .parse::<SocketAddr>()
        .unwrap();
    let parent = settings
        .get("parent")
        .and_then(|addr| addr.parse::<SocketAddr>().ok());

    // TODO: Spawn up device monitors

    // IDEA: Maybe spawn up device modalities
        // Using the plugin architecture, I could forward the path to some launcher script
            // The scripts would then send messages back to this manager in order to negotiate behavior
        // Not sure how to get this right in generality (maybe use the config watcher example?)

    // Spawn up the server
    internal::Server::new().spawn(addr, parent);
}

// Unused because I don't want to have the config files outside of the development dir just yet
const APP_INFO: app_dirs::AppInfo = app_dirs::AppInfo {
    name: "personal-AI-device_manager",
    author: "Grayson Hooper"
};

// API Documentation:
//  tokio-serde-json: https://github.com/carllerche/tokio-serde-json
