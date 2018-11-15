
use std::convert;
use std::net::IpAddr;

use serde_json;

// TODO: This doesn't allow any other fields than what I've specified
// How would I be able to get a view on the aspects of the message that I care about?
// Would that even be possible if I want to modify it (which I do)

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    // routing
    pub message_id: String,
    pub parent_id: Option<String>,
    pub send_ack: bool,
    pub route: Vec<IpAddr>,
    pub forward: Option<bool>,
    pub sender: MessageSender,
    pub dest: MessageDest,

    // TODO: Need to figure out what I'm currently using
    // TODO: Need to define the message system based off of Rust
    // TODO: This package will need to be made into a separate crate
    // package data
    pub action: Option<String>,
    pub args: Option<Vec<serde_json::Value>>,
    pub resp: Option<serde_json::Value>,
    pub body: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessageSender {
    pub uuid: Option<String>,
    pub role: Option<String>,
    pub addr: Option<IpAddr>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessageDest {
    pub broadcast: Option<bool>,
    pub role: Option<String>,
    pub addr: Option<IpAddr>,
    pub uuid: Option<String>,
    pub intra_device: Option<bool>
}

impl Message {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self{
            message_id: "".to_string(),
            parent_id: None,
            send_ack: false,
            route: Vec::new(),
            forward: None,
            sender: MessageSender{
                uuid: None,
                role: None,
                addr: None,
            },
            dest: MessageDest{
                broadcast: None,
                role: None,
                addr: None,
                uuid: None,
                intra_device: None
            },
            action: None,
            args: None,
            resp: None,
            body: None,
        }
    }
}

impl convert::Into<MessageDest> for MessageSender {
    fn into(self) -> MessageDest {
        MessageDest{
            broadcast: None,
            role: self.role,
            addr: self.addr,
            uuid: self.uuid,
            intra_device: None
        }
    }
}
