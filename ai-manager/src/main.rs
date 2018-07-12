
extern crate tokio;
#[macro_use] extern crate serde_json;
extern crate futures;

extern crate clap;
#[macro_use] extern crate log;
extern crate fern;
extern crate chrono;

extern crate server;

mod device;
mod serve;

// This program acts as the interaction manager for the ai system,
// Collecting and dispatching requests from the individual devices
// And intermediating between them and the ai modality programs

// TODO: Implement this with the same degree of accuracy that I have with device-manager

fn main() {
    println!("Hello, world!");
}
