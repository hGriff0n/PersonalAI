extern crate tokio;
extern crate tokio_io;
extern crate tokio_tcp;
extern crate tokio_serde_json;

#[macro_use]
extern crate serde_json;

use std::net::SocketAddr;

use tokio::prelude::*;
use tokio_tcp::TcpStream;
// use tokio_io::AsyncRead;
// use tokio_io::codec::LinesCodec;
use tokio_io::codec::length_delimited;

use serde_json::Value;
use tokio_serde_json::*;

// NOTE: This is currently "misnamed"/"misimplemented"
// This program currently acts as a client, not a true server
// I'm currently developing the device-manager, though
fn main() {
    // Parse what address we're going to connect to
    let addr = "127.0.0.1:6142".parse::<SocketAddr>().unwrap();

    let client = TcpStream::connect(&addr)
        .and_then(|conn| {
            // Delimit frames using a length header
            let length_delimited = length_delimited::FramedWrite::new(conn);

            // Serialize frames with JSON
            let serialized = WriteJson::<_, Value>::new(length_delimited);

            serialized.send(json!({"text": "hello" }))
                .and_then(|_conn| Ok(()))
        })
    // Connect to the tcp server
    // The issue may be the server still considers this stream to be open
    // Maybe I should convert to UDP/RUDP for my connections
    // let client = TcpStream::connect(&addr)
    //     .and_then(|conn| {\
    //         // let (writer, reader) = length_delimited::Framed::new(conn).split();
    //         // let writer = WriteJson::new(writer);
    //         // let _reader = ReadJson::new(reader);

    //         // writer.send(json!({ "text": "Hello" }))
    //             // .and_then(|conn| )
    //         let framed = conn.framed(LinesCodec::new());
    //         framed.send("Hello!".to_string())
    //             .and_then(|conn| {
    //                 conn.for_each(|line| {
    //                     println!("Received line {}", line);
    //                     Ok(())
    //                 })
    //             })


    //     })
        .map_err(|err| println!("Stream error: {:?}", err));

    tokio::run(client)
}
