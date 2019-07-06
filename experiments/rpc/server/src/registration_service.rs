
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
rpc_schema!(RegisterServerArgs {
    handles: Vec<String>
});

rpc_schema!(RegisterServerResponse {
    registered: Vec<String>
});

rpc_service! {
    RegistrationService<protocol::JsonProtocol>

    rpc register_server(self, args: RegisterServerArgs) -> RegisterServerResponse {
        RegisterServerResponse{
            registered: args.handles
        }
    }
}
