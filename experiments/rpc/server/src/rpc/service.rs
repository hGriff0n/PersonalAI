
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
    fn register_endpoints<R: Registry<P>>(self, register: &R);
}

// Alternative trait for allowing registration of rpc services
pub trait Registry<P: protocol::RpcSerializer> {
    fn register(&self, fn_name: &str, callback: Box<types::Function<P>>);
    // TODO(r/41517): Improve once trait aliases are in stable
    fn register_fn<F>(&self, fn_name: &str, callback: F)
        where F: Fn(types::Message) -> types::Result<P::Message> + Send + Sync + 'static
    {
        self.register(fn_name, Box::new(callback))
    }
}
