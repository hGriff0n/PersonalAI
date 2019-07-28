// location to try out rpcs and other stuff

// standard imports
use std;

// third-party imports
use serde::{Serialize, Deserialize};

// local imports
use crate::protocol;
#[allow(unused_imports)] use crate::rpc;

//
// Implementation
//

pub struct ExperimentalService {
    // TODO: This won't actually work at tracking num active connections because the service isn't told about that
    // active_conns: std::sync::atomic::AtomicU32,
}

impl ExperimentalService {
    pub fn new() -> Self {
        Self{
            // active_conns: std::sync::atomic::AtomicU32::new(0),
        }
    }
}

//
// RpcService Definition
//

// TODO: Look into possibility of adding extra schema information/etc.
rpc_schema!(ActiveConnsResponse {
    registered: u32
});

rpc_service! {
    ExperimentalService<protocol::JsonProtocol>

    rpc num_active_connections(self, _caller, _args) -> ActiveConnsResponse {
        futures::future::ok(ActiveConnsResponse{
            registered: 0,
        })
    }
}
