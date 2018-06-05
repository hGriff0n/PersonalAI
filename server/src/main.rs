extern crate tokio;
extern crate tokio_io;
extern crate tokio_tcp;
extern crate tokio_serde_json;
#[macro_use] extern crate serde_json;

mod comm;

use std::net::SocketAddr;

use tokio::prelude::*;
use tokio_tcp::TcpStream;
use tokio_io::codec::length_delimited::Framed;

use serde_json::Value;
use tokio_serde_json::*;

// NOTE: This is currently "misnamed"/"misimplemented"
// This program currently acts as a client, not a true server
// I'm currently developing the device-manager, though
fn main() {
    // Parse what address we're going to connect to
    let addr = "127.0.0.1:6142".parse::<SocketAddr>().unwrap();

    let message = std::env::args().nth(1).unwrap_or("hello".to_string());

    // Connect to the tcp server
    let action = TcpStream::connect(&addr)
        .and_then(move |conn| {
            // Split the connection into reader and writer
            let (writer, reader) = Framed::new(conn).split();
            let writer = WriteJson::<_, Value>::new(writer);
            let reader = ReadJson::<_, Value>::new(reader);

            // NOTE: I don't need the "stop" channel as this is a "client" communicator
            // The final implementation will have it as the structure is ultimately recursive

            // Setup the communication channel
            let (sink, source) = std::sync::mpsc::channel::<Value>();
            let source = comm::FutureChannel::new(source);

            // Unilaterally send a message to the server
            sink.send(json!({ "text": message })).unwrap();

            // Define the reader action
            let read_action = reader
                .for_each(move |msg| {
                    println!("Received {:?}", msg);
                    sink.send(json!({ "action": message })).unwrap();
                    Ok(())
                });

            #[allow(unused_mut)]
            // Define the writer action
            let write_action = writer
                .send_all(source.transform(move |msg| msg));

            // Assemble the actions into a single "tokio" packet
            let action = read_action
                .select2(write_action)
                // .select2(cancel)                             // NOTE: This needs to come last in order for it to work
                .map(|_| {})
                .map_err(|_| ());                               // NOTE: I'm ignoring all errors for now

            // Finally spawn the connection
            tokio::spawn(action);
            Ok(())
        })
        .map_err(|err| {
            println!("Server error: {:?}", err)
        });

    // Start the server and tokio runtime
    tokio::run(action);
}

// API Documentation:
//  tokio-serde-json: https://github.com/carllerche/tokio-serde-json
