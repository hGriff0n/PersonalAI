extern crate tokio;
extern crate tokio_tcp;
extern crate tokio_io;

use std::net::SocketAddr;

use tokio_tcp::TcpStream;
use tokio::prelude::*;
use tokio_io::AsyncRead;
use tokio_io::codec::LinesCodec;


// TODO:
// Figure out how to close the session manually
// Simplify and improve the "server" implementation (make sending and receiving as nice as possible)
// Work out better implementation and communication
fn main() {
    // Parse what address we're going to connect to
    let addr = "127.0.0.1:6142".parse::<SocketAddr>().unwrap();

    // Connect to the tcp server
    let client = TcpStream::connect(&addr)
        .and_then(|conn| {
            let framed = conn.framed(LinesCodec::new());
            framed.send("Hello!".to_string())
                  .and_then(|conn| {
                      conn.for_each(|line| {
                        println!("Received line {}", line);
                        Ok(())
                    })
                  })
        })
        .map_err(|err| println!("Stream error: {:?}", err));

    tokio::run(client)
}
