
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

rpc_service! {
    FortuneService<protocol::JsonProtocol>

    rpc tell_fortune(self, _caller, args: TellFortuneArgs) -> TellFortuneResponse {
        let fortune = self.generate_fortune(args.sign.as_str());
        TellFortuneResponse{fortune: fortune}
    }

    rpc fake_fortune(self, _caller, _args: TellFortuneArgs) -> TellFortuneResponse {
        TellFortuneResponse{fortune: "Bah".to_string()}
    }
}
