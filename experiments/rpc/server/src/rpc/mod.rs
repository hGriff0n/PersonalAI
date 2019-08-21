
pub mod dispatch;
mod service;
#[macro_use] mod types;  // NOTE: The `[macro_use]` is required to get access to `rpc_schema!`

pub use service::*;
pub use types::*;

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
    ($type:ty) => { stringify!($type) };
}

// Helper method to extract rpc arguments from the rpc network message
// This attempts to parse the `args` value to the defined type, returning an error if unable
// If no argument type is specified, this enforces that no arguments were passed in (TODO: Keep?)
// TODO: The error generation for no-args explicitly references std::io::Error (maybe problematic)
#[doc(hidden)]
#[macro_export]
macro_rules! __typecast_rpc_args {
    ($proto:ty | $call_msg:ident) => {{
        let _msg = $call_msg;
    }};
    ($proto:ty | $call_msg:ident $arg_type:ty) => {{
        let args = <$proto as $crate::protocol::RpcSerializer>::from_value::<$arg_type>($call_msg.args);
        // TODO: Remove when the endpoint code starts returning my error format
        if let Err(err) = args {
            let io_err = std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{}", err));
            return Box::new(futures::future::err(io_err));
        }
        args.unwrap()
    }};
}

// Helper macro to generate return codes for rpc endpoints
// This is necessary to handle cases where no return type was specified (ie. no response)
#[doc(hidden)]
#[macro_export]
macro_rules! __wrap_rpc_return {
    // NOTE: The `|` is required because $protocol is a type
    ($protocol:ty | $rpc_resp:ident) => {
        $rpc_resp.and_then(|_resp| futures::future::ok(None))
    };
    ($protocol:ty | $rpc_resp:ident $resp_type:ty) => {
        $rpc_resp.and_then(|resp| futures::future::ok(
            Some(<$protocol as $crate::protocol::RpcSerializer>::to_value::<$resp_type>(resp).unwrap())
        ))
    };
}

// Helper macro to add in the correct code to type check the rpc definition the way the user defined it
// No response handles should return a future of `()`
#[doc(hidden)]
#[macro_export]
macro_rules! __wrap_user_body {
    ($fn_body:block) => {{
        let tmp: futures::future::FutureResult<(), std::io::Error> = $fn_body;
        tmp
    }};
    ($fn_body:block $resp_ty:ty) => {{
        let tmp: futures::future::FutureResult<$resp_ty, std::io::Error> = $fn_body;
        tmp
    }};
}

// TODO: Figure out a way to allow for renaming handles (attributes?)
// Macro that defines and implements an rpc service
// Defined rpcs are automatically wrapped with correct argument parsing and response handling code
// NOTE: Rust allows for multiple `impl $service` blocks
    // These can be used to define constructors and other helper methods
// NOTE: I don't quite like the implicit dependency on some type defs this has
// NOTE: For reporting errors, use this recipe: "Err(...)?"
#[macro_export(local_inner_macros)]
macro_rules! rpc_service {
    // generate_args, ignore_args_if_none, and generate_return only operate on 0 or 1 "arguments"
    // This prevents any '*' usage in the macro from allowing 2 matches so we use it to mimic a regex `?`
    (
        $service:ident<$proto:ty>
        $(
            $(#[$_:meta])*  // Match any "attribute" specified on the rpc (unused, alt syntax: `$(@$_:meta)*`)
            rpc $name:ident($this:ident, $caller:ident, $args:ident $(: $arg_type:ty)*) $(-> $resp_ty:ty)* $fn_body:block
        )*
    ) => {
        impl $service {
            $(
                fn $name(&$this, $caller: std::net::SocketAddr, call_msg: $crate::rpc::Message)
                    -> Box<dyn futures::Future<Item=Option<<$proto as $crate::protocol::RpcSerializer>::Message>, Error=std::io::Error> + Send>
                {
                    let $args = __typecast_rpc_args!($proto | call_msg $($arg_type)*);
                    use futures::Future;
                    let rpc_resp = __wrap_user_body!($fn_body $($resp_ty)*);

                    let _caller = $caller;
                    __silence_unused_args_warning!($args $($arg_type)*);
                    Box::new(__wrap_rpc_return!($proto | rpc_resp $($resp_ty)*))
                }
            )*
        }

        // Setup the registration for the rpc calls
        impl $crate::rpc::Service<$proto> for $service {
            fn register_endpoints<R: $crate::rpc::Registry<$proto>>(self, register: &R) -> Result<(), String> {
                let service = std::sync::Arc::new(self);

                $({
                    let endpoint_server = service.clone();
                    if !register.register_fn(
                        __stringify!($name),
                        move |caller: std::net::SocketAddr, msg: $crate::rpc::Message| endpoint_server.$name(caller, msg))
                    {
                        return Err(
                            std::format!(
                                "Error when registering handle {}::{} - handle already exists",
                                __stringify!($service), __stringify!($name)));
                    }
                })*

                Ok(())
            }
        }
    }
}
