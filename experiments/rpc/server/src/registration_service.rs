
// standard imports

// third-party imports
use serde::{Serialize, Deserialize};

// local imports
use crate::protocol;
#[allow(unused_imports)] use crate::rpc;

//
// Implementation
//

pub struct RegistrationService;

impl RegistrationService {
    pub fn new() -> Self {
        Self{}
    }
}

//
// RpcService Definition
//

// TODO: Look into possibility of adding extra schema information/etc.
rpc_schema!(RegisterAppArgs {
    handles: Vec<String>
});

rpc_schema!(RegisterAppResponse {
    registered: Vec<String>
});

rpc_service! {
    RegistrationService<protocol::JsonProtocol>

    rpc register_app(self, _caller, args: RegisterAppArgs) -> RegisterAppResponse {
        let mut registered = Vec::new();
        for handle in args.handles {
            // TODO: Register the handle in the Dispatcher
            registered.push(handle);
        }
        RegisterAppResponse{
            registered: registered
        }
    }

    // rpc list_books(self, _args) -> RegisterAppResponse {
    //     RegisterAppResponse{
    //         registered: vec![ "Dune" ]
    //     }
    // }
}
