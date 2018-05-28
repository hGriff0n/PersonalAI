extern crate tokio;
extern crate tokio_io;
extern crate tokio_tcp;
extern crate tokio_serde_json;

#[macro_use]
extern crate serde_json;

use std::net::SocketAddr;

use tokio::prelude::*;
use tokio_tcp::TcpStream;
use tokio_io::codec::length_delimited;

use serde_json::Value;
use tokio_serde_json::*;

// NOTE: This is currently "misnamed"/"misimplemented"
// This program currently acts as a client, not a true server
// I'm currently developing the device-manager, though
fn main() {
    // Parse what address we're going to connect to
    let addr = "127.0.0.1:6142".parse::<SocketAddr>().unwrap();

    // Connect to the tcp server
    let client = TcpStream::connect(&addr)
        .and_then(|conn| {
            let json = WriteJson::<_, Value>::new(length_delimited::Framed::new(conn));

            // Rotate between the reader and the writer connection
            json.send(json!({"text": "hello" }))
                .and_then(|conn| {
                    let conn = ReadJson::<_, Value>::new(conn.into_inner());
                    conn.for_each(|line| {
                        println!("Received {:?}", line);
                        Ok(())
                    })
                })
        })
        .map_err(|err| println!("Stream error: {:?}", err));

    tokio::run(client)
}
