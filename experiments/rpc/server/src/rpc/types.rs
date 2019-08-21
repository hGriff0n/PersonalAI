
// standard imports
use std;

// third-party imports
use serde::{Serialize, Deserialize};

// local imports
use crate::protocol;


//
// Implementation
//

// Helper macro to automatically create a serde struct of the given elements
// This struct will be used by rpc endpoints to define arguments and return values
// TODO: Is there a way to enable `rpc_schema! Type { Fields* }`?
// TODO: Is there a way to use this as `rpc_macros::rpc_schema!`?
#[macro_export]
macro_rules! rpc_schema {
    ($name:ident { $($arg:ident: $type:ty),+ }) => {
        rpc_schema!($name, $($arg: $type),+);
    };
    ($name:ident, $($arg:ident: $type:ty),+) => {
        #[derive(Clone, Debug, Serialize, Deserialize)]
        pub struct $name {
            $(pub $arg: $type,)+
        }
    }
}

// Entrypoint schema for all rpc messages through network channels (we only send and receive this type)
// TODO: Make the `args` and `resp` generalized on the protocol message type
// They still bring in a little dependency on the specific protocol being used (may be a good thing)
rpc_schema!(Message {
    // Call communication
    // The individual handles will be responsible for implementing serialization of args+resp
    call: String,
    args: <protocol::JsonProtocol as protocol::RpcSerializer>::Message,
    resp: Option<<protocol::JsonProtocol as protocol::RpcSerializer>::Message>,

    // Call metadata
    msg_id: uuid::Uuid
});

impl Message {
    #[allow(dead_code)]
    fn new(call: String, args: <protocol::JsonProtocol as protocol::RpcSerializer>::Message) -> Self {
        Self{
            call: call,
            args: args,
            resp: None,
            msg_id: uuid::Uuid::new_v4(),
        }
    }
}


// TODO: Improve typing usage and genericity
// TODO: Utilize an "RpcError" type
pub type Result<T> = Box<dyn futures::Future<Item=Option<T>, Error=std::io::Error> + Send>;
pub type Function<P> = dyn Fn(std::net::SocketAddr, Message) -> self::Result<<P as protocol::RpcSerializer>::Message> + Send + Sync;

// TODO: trait aliases are experimental (https://github.com/rust-lang/rust/issues/41517)
// NOTE: Currently we can't use `F: impl Function<P>` or `where F: Function<P>` in some definitions that accept closures
// We're instead forced to "reimplement" the definition, ie. `where F: Fn(Message) -> ...`
// This is apparently because `Function` isn't a trait, it's a type - and Rust requires traits in those situations
// pub trait FnType<P: protocol::RpcSerializer> = Fn(Message) -> self::Result<<P as protocol::RpcSerializer>::Message> + Send + Sync;
