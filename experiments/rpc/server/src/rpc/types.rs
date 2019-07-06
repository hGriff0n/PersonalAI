
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
        #[derive(Clone, Serialize, Deserialize)]
        pub struct $name {
            $(pub $arg: $type,)+
        }
    };
    ($name:ident, $($arg:ident: $type:ty),+) => {
        #[derive(Clone, Serialize, Deserialize)]
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
    msg_id: String,
    app_id: String
});


// TODO: Improve typing usage and genericity
// TODO: Utilize an "RpcError" type
pub type Result<T> = std::result::Result<Option<T>, std::io::Error>;
pub type Function<P> = Fn(Message) -> self::Result<<P as protocol::RpcSerializer>::Message> + Send + Sync;
