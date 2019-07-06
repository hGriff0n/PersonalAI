
pub mod json;
mod types;

pub use types::*;
use crate::protocol;

// Helper macro to generate return codes for rpc endpoints
// This is necessary to handle cases where no return type was specified (ie. no response)
#[doc(hidden)]
#[macro_export]
macro_rules! __wrap_rpc_return {
    // NOTE: The `|` is required because $protocol is a type
    ($protocol:ty | $resp:ident) => {{
        let _resp = $resp;      // Silence any warnings about "unused variables"
        Ok(None)
    }};
    ($protocol:ty | $resp:ident $arg_resp:ty) => {
        Ok(Some(<$protocol>::to_value::<$arg_resp>($resp)?))
    }
}


// Helper method to extract rpc arguments from the rpc network message
// This attempts to parse the `args` value to the defined type, returning an error if unable
// If no argument type is specified, this enforces that no arguments were passed in (TODO: Keep?)
// TODO: The error generation for no-args explicitly references std::io::Error (maybe problematic)
#[doc(hidden)]
#[macro_export]
macro_rules! __typecheck_rpc_args {
    ($protocol:ty | $args:ident) => {{
        if !$args.args.is_null() {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "arguments given to non-arg rpc"));
        }

        $args
    }};
    ($protocol:ty | $args:ident $arg_type:ty) => {
        <$protocol>::from_value::<$arg_type>($args.args)?
    }
}


// Helper macro to silence "unused_variable" warnings when handling no-arg rpcs
// We do this trough assignment as that uses the variable and has an "effect"
// NOTE: Not having an "effect" which would produce a *different* warning
#[doc(hidden)]
#[macro_export]
macro_rules! __silence_unused_args_warning {
    ($args:ident) => {{ let _args = $args; }};
    ($args:ident $arg_type:ty) => {}
}

// Helper method to wrap `stringify!` - cause that's needed for some reason
#[doc(hidden)]
#[macro_export]
macro_rules! __stringify {
    ($name:ident) => { stringify!($name) };
}

// TODO: Figure out a way to allow for renaming handles (attributes?)
// TODO: Allow for specifying attributes on rpc?
// Macro that defines and implements an rpc service
// Defined rpcs are automatically wrapped with correct argument parsing and response handling code
// NOTE: Rust allows for multiple `impl $service` blocks
    // These can be used to define constructors and other helper methods
// NOTE: I don't quite like the implicit dependency on some type defs this has
#[macro_export(local_inner_macros)]
macro_rules! rpc_service {
    // generate_args, ignore_args_if_none, and generate_return only operate on 0 or 1 "arguments"
    // This prevents any '*' usage in the macro from allowing 2 matches so we use it to mimic a regex `?`
    ($service:ident<$proto:ty> $(rpc $name:ident($this:ident, $args:ident $(: $arg_type:ty)*) $(-> $arg_resp:ty)* $fn_body:block)*)
    => {
        impl $service {
            $(
                fn $name(&$this, $args: $crate::rpc::Message) -> $crate::rpc::Result<<$proto as $crate::protocol::RpcSerializer>::Message> {
                    let $args = __typecheck_rpc_args!($proto | $args $($arg_type)*);
                    __silence_unused_args_warning!($args $($arg_type)*);
                    let resp = $fn_body;
                    __wrap_rpc_return!($proto | resp $($arg_resp)*)
                }
            )*
        }

        // Setup the registration for the rpc calls
        impl $crate::rpc::Service<$proto> for $service {
            fn endpoints(self) -> Vec<(String, Box<$crate::rpc::Function<$proto>>)> {
                let service = std::sync::Arc::new(self);

                let mut endpoints: Vec<(String, Box<$crate::rpc::Function<$proto>>)> = Vec::new();
                $(
                    {
                        let endpoint_server = service.clone();
                        endpoints.push((
                            __stringify!($name).to_string(),
                            Box::new(move |msg: $crate::rpc::Message| endpoint_server.$name(msg))));
                    }
                )*
                endpoints
            }

            // Alternate method of registering rpc endpoints
            // This relies on passing in a registration object and then calling `register` for each endpoint
            // fn register_endpoints<R: Registry>(self, register: &R) {
            //     let service = std::sync::Arc::new(self);
            //     $({
            //         let endpoint_server = service.clone();
            //         register.register(stringify!($name), move |msg: RpcMessage| endpoint_server.$name(msg));
            //     })*
            // }
        }
    };
}



// TODO: Move these into a separate file?
// Trait that defines the way rpc services export rpc endpoint handles
pub trait Service<P: protocol::RpcSerializer> {
    fn endpoints(self) -> Vec<(String, Box<Function<P>>)>;
    // fn register_endpoints<R: Registry>(self, register: &R);
}

// Alternative trait for allowing registration of rpc services
// trait Registry {
//     fn register<F>(&self, fn_name: &str, callback: F)
//         where F: Fn(Message) -> json::Result + Send + Sync + 'static;
// }
