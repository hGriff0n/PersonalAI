
// standard imports

// third-party imports
use serde::{Serialize, Deserialize};

// local imports
use crate::protocol;
#[allow(unused_imports)] use crate::rpc;

//
// Implementation
//

pub struct FortuneService;

impl FortuneService {
    pub fn new() -> Self {
        Self{}
    }

    fn generate_fortune(&self, sign: &str) -> String {
        match &sign {
            &"leo" => "latin for lion".to_string(),
            sign => format!("Horoscope unimplemented for sign '{}'", sign)
        }
    }
}

//
// RpcService Definition
//

rpc_schema!(TellFortuneArgs {
    sign: String
});
rpc_schema!(TellFortuneResponse {
    fortune: String
});

// NOTE: This service exists more to check the macro generation code works
// This has rpcs that test all 4 combinations of arg and return types
rpc_service! {
    FortuneService<protocol::JsonProtocol>

    rpc tell_fortune(self, _caller, args: TellFortuneArgs) -> TellFortuneResponse {
        let fortune = self.generate_fortune(args.sign.as_str());
        futures::future::ok(TellFortuneResponse{fortune: fortune})
    }

    rpc fake_fortune(self, _caller, _args) -> TellFortuneResponse {
        futures::future::ok(TellFortuneResponse{fortune: "Bah".to_string()})
    }

    rpc deaf_fortune(self, _caller, args: TellFortuneArgs) {
        let _args = args;
        futures::future::ok(())
    }

    rpc ask_for_more_wishes(self, _caller, _args) {
        futures::future::ok(())
    }
}
