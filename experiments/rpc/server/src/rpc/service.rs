
// standard imports

// third-party imports

// local imports
use super::types;
use crate::protocol;

//
// Implementation
//

// TODO: Move these into a separate file?
// Trait that defines the way rpc services export rpc endpoint handles
pub trait Service<P: protocol::RpcSerializer> {
    fn endpoints(self) -> Vec<(String, Box<types::Function<P>>)>;
    // fn register_endpoints<R: Registry>(self, register: &R);
}

// Alternative trait for allowing registration of rpc services
// trait Registry<P: protocol::RpcSerializer> {
//     fn register<F>(&self, fn_name: &str, callback: F)
//         where F: Fn(types::Message) -> types::Result<P> + Send + Sync + 'static;
// }
