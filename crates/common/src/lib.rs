use serde::{Deserialize, Serialize};

/// Commands sent from the Client to the Server
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Command {
    SubscribeBroadcast {
        channel: String,

        // Default this to false
        #[serde(default)]
        subscribe_to_self: bool,
    },
    UnsubscribeBroadcast(String),
    SendBroadcast {
        channel: String,
        message: String,
    },
    SetString {
        key: String,
        value: String,
        expiration: Option<u32>,
    },
    GetString {
        key: String,
    },
}

/// Messages sent from the Server to Clients
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Message {
    ClientId(String),
    BroadcastMessage(BroadcastMessage),
    GetString(Option<String>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BroadcastMessage {
    pub client_id: String,
    pub channel: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Remove this testing func
    #[test]
    fn serialize() {
        let item = BroadcastMessage {
            client_id: "test".to_string(),
            channel: "test".to_string(),
            message: "testing message".to_string(),
        };
        let item_enum = Message::BroadcastMessage(BroadcastMessage {
            client_id: "test".to_string(),
            channel: "test".to_string(),
            message: "testing message".to_string(),
        });
        let json = serde_json::to_string(&item);
        let json_enum = serde_json::to_string(&item_enum);
        println!("json: {:?}", json);
        println!("json enum: {:?}", json_enum);
        assert_eq!(4, 4);
    }
}
