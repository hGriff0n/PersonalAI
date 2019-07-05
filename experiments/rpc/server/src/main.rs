
// #[macro_use] extern crate log;
mod protocol;
mod rpc_macros;

// macro imports
// use serde_json::json;

// standard imports
use std::net;

// third-party imports
use serde::{Serialize, Deserialize};
use tokio::prelude::*;

// local imports
use crate::protocol::*;


//
// Implementation
//

//
// Global rpc communication types
//

// Entrypoint schema for all rpc messages through network channels (we only send and receive this type)
// TODO: Make the `args` and `resp` generalized on the protocol message type
// They still bring in a little dependency on the specific protocol being used (may be a good thing)
rpc_schema!(RpcMessage {
    // Call communication
    // The individual handles will be responsible for implementing serialization of args+resp
    call: String,
    args: serde_json::Value,
    resp: Option<serde_json::Value>,

    // Call metadata
    msg_id: String,
    app_id: String
});

// TODO: Improve typing usage and genericity
// TODO: Utilize an "RpcError" type
type RpcResult<T> = Result<Option<T>, std::io::Error>;
type RpcFunction<P> = Fn(RpcMessage) -> RpcResult<<P as protocol::RpcSerializer>::Message> + Send + Sync;

// Overloads for the current json protocol
// NOTE: These are fine to use directly as we only currently support json anyways (type schemes a bit hard to disentangle)
#[allow(dead_code)]
type JsonRpcResult = RpcResult<<protocol::JsonProtocol as protocol::RpcSerializer>::Message>;
#[allow(dead_code)]
type JsonRpcFunction = RpcFunction<protocol::JsonProtocol>;


// Define an entry point for services to register there methods
// trait Registry {
//     fn register<F>(&self, fn_name: &str, callback: F)
//         where F: Fn(RpcMessage) -> JsonRpcResult + Send + Sync + 'static;
// }
trait RegistratableService {
    fn endpoints(self) -> Vec<(String, Box<JsonRpcFunction>)>;
    // fn register_endpoints<R: Registry>(self, register: &R);
}


//
// RPC Services Definitions
//

rpc_schema!(TellFortuneArgs {
    sign: String
});
rpc_schema!(TellFortuneResponse, fortune: String);

// TODO: Figure out a good way to construct services with arguments
// NOTE: Services should use interior mutability (passing everything as &self)
struct HelloServiceAlt;

impl HelloServiceAlt {
    pub fn new() -> Self {
        Self{}
    }

    fn generate_fortune(&self, sign: &str) -> String {
        match &sign {
            &"leo" => "latin for lion".to_string(),
            sign => format!("Horoscope unimplemented for sign '{}'", sign)
        }
    }
}

rpc_service! {
    HelloServiceAlt<protocol::JsonProtocol>

    rpc tell_fortune(self, args: TellFortuneArgs) -> TellFortuneResponse {
        let fortune = self.generate_fortune(args.sign.as_str());
        TellFortuneResponse{fortune: fortune}
    }

    rpc fake_fortune(self, args: TellFortuneArgs) -> TellFortuneResponse {
        let _args = args;
        TellFortuneResponse{fortune: "Bah".to_string()}
    }

    // No returns are handled by not sending a response back
    // rpc no_fortune(self, args: TellFortuneArgs) {
    //     let _args = args;
    // }

    // You don't even need to accept any arguments
    // rpc request_services(self, args) {
    //     println!("Hello");
    // }
}

#[derive(Clone)]
struct RpcDispatcher {
    handles: std::sync::Arc<
        std::sync::RwLock<
            std::collections::HashMap<String, Box<Fn(RpcMessage) -> JsonRpcResult + Send + Sync>>>>,
}

impl RpcDispatcher {
    pub fn new() -> Self {
        Self{
            handles: std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub fn add_service<S: RegistratableService>(self, service: S) -> Self {
        // service.register_endpoints(&self);
        for (endpoint, callback) in service.endpoints() {
            self.handles
                .write()
                .unwrap()
                .insert(endpoint.to_string(), callback);
        }
        self
    }

    // TODO: Integrate this with tokio/futures a little bit better
    fn dispatch(&self, mut rpc_call: RpcMessage) -> Option<RpcMessage> {
        match self.handles
            .read()
            .unwrap()
            .get(&rpc_call.call)
            .and_then(|handle|
                Some(handle(rpc_call.clone())))
        {
            // Call succeded, no response
            Some(Ok(None)) => None,

            // Call succeded, repsonse
            Some(Ok(Some(resp))) => {
                rpc_call.resp = Some(resp);
                Some(rpc_call)
            },

            // Call failed
            Some(Err(_err)) => {
                rpc_call.resp = Some(JsonProtocol::to_value("Error: Error in rpc handler").unwrap());
                Some(rpc_call)
            },

            // Handle not registered
            None => {
                rpc_call.resp = Some(JsonProtocol::to_value("Error: Invalid rpc call").unwrap());
                Some(rpc_call)
            }
        }
    }
}

//
// Server running code
//

fn main() {
    let addr = "127.0.0.1:6142".parse::<net::SocketAddr>()
        .expect("Failed to parse hardcoded socket address");

    // let device_manager = DeviceManager::new();
    let rpc_dispatcher = RpcDispatcher::new()
        .add_service(HelloServiceAlt::new())
        ;

    serve(rpc_dispatcher, addr);
}

// TODO: Improve error handling?
fn serve(dispatcher: RpcDispatcher, addr: std::net::SocketAddr) {
    // Current protocols don't require state, so we currently access it statically
    // TODO: Need a way to ensure we're all using the same protocol
    type P = JsonProtocol;

    let server = tokio::net::TcpListener::bind(&addr)
        .expect("Failed to bind server to specified sock address")
        .incoming()
        .map_err(|err| eprintln!("Failed to accept incoming connection: {:?}", err))
        .for_each(move |conn| {
            let _peer = conn.peer_addr().expect("Failed to extract peer address from TcpStream");

            // Construct the communication frames
            let (reader, writer) = frame_with_protocol::<P, _, _>(conn, &|| tokio::codec::LengthDelimitedCodec::new());
            let writer = writer.sink_map_err(|err| eprintln!("error in json serialization: {:?}", err));

            // Construct communication channels between the read and write ends
            // This segments the control flow on the two ends, making the stream processing a little nicer
            let (sender, receiver) = futures::sync::mpsc::unbounded();
            // TODO: Add in close channel? Closing it on the client's end automatically closes it on the server's end

            // Receive a message from the connection and dispatch it to the rpc service
            let rpc_dispatcher = dispatcher.clone();
            let read_action = reader
                .for_each(move |msg| {
                    // TODO: Figure out how to make this asynchronous?
                    // Marshal the call off to the rpc dispatcher
                    if let Some(msg) = rpc_dispatcher.dispatch(P::from_value(msg)?) {
                        // If a response was produced send it back to the caller
                        sender.unbounded_send(msg)
                            .map_err(|_err|
                                std::io::Error::new(std::io::ErrorKind::Other, "Failed to send message through pipe"))

                    } else {
                        Ok(())
                    }
                })
                .map(|_| ())
                .map_err(|err| eprintln!("Error: {:?}", err));

            // Reformat rpc responses and send them back on the line
            let write_action = receiver
                .map(move |msg| P::to_value(msg).unwrap())
                .forward(writer)
                .map(|_| ())
            .map_err(|err| eprintln!("socket write error: {:?}", err));

            // Spawn the actions in tokio
            let action = read_action
                .select2(write_action)
                .map(move |_| println!("Closed connection with {:?}", addr))
                .map_err(|_| eprintln!("close connection error"));
            tokio::spawn(action)
        });

    tokio::run(server);
}
