use super::types;
use crate::protocol;

// Overloads for the current json protocol
// NOTE: These are fine to use directly as we only currently support json anyways (type schemes a bit hard to disentangle)
pub type Result = types::Result<<protocol::JsonProtocol as protocol::RpcSerializer>::Message>;
#[allow(dead_code)]
pub type Function = types::Function<protocol::JsonProtocol>;
