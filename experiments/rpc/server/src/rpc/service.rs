
// standard imports
use std::net;

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
    // Register all exported by the service in the passed Registry object
    // Returns a Result indicating any error message produced during registration
    // NOTE: This (currently) only happens when a handle fails to register due to name clashes
        // The returned string is an error string for reporting which handle caused the collision
    fn register_endpoints<R: Registry<P>>(self, register: &R) -> Result<(), String>;
}

// Alternative trait for allowing registration of rpc services
pub trait Registry<P: protocol::RpcSerializer> {
    fn register(&self, fn_name: &str, callback: Box<types::Function<P>>) -> bool;
    // TODO(r/41517): Improve once trait aliases are in stable
    fn register_fn<F>(&self, fn_name: &str, callback: F) -> bool
        where F: Fn(net::SocketAddr, types::Message) -> types::Result<P::Message> + Send + Sync + 'static
    {
        self.register(fn_name, Box::new(callback))
    }
}
