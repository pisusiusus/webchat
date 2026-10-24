use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, PartialEq)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum IncomingMessage {
    Login { name: String },

    Chat { text: String },
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum OutgoingMessage {
    Welcome {
        id: u64,
        name: String,
    },

    Chat {
        sender_id: u64,
        sender_name: String,
        text: String,
        room: String,
        timestamp: u64,
    },

    Error {
        code: ErrorCode,
        message: String,
    },
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidJson,
    NotLoggedIn,
    AlreadyLoggedIn,
    InvalidName,
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_login_json() {
        let login_message = IncomingMessage::Login {
            name: "Alice".to_string(),
        };

        let own_message = r#"{"type":"login", "name":"Alice"}"#;

        let json_data: IncomingMessage =
            serde_json::from_str(own_message).expect("JSON konnte nicht geladen werden.");

        assert_eq!(login_message, json_data);
    }

    #[test]
    fn test_incoming_message_chat() {
        let chat_message = IncomingMessage::Chat {
            text: "Hallo".to_string(),
        };

        let own_message = r#"{"type":"chat", "text":"Hallo"}"#;

        let json_data: IncomingMessage =
            serde_json::from_str(own_message).expect("JSON could not be read.");

        assert_eq!(chat_message, json_data);
    }

    #[test]
    fn test_unknown_type() {
        let own_message = r#"{"type":"unknown", "text":"Hallo"}"#;

        let json_data: Result<IncomingMessage, serde_json::Error> =
            serde_json::from_str(own_message);

        assert!(json_data.is_err());
    }

    #[test]
    fn test_broken_json() {
        let own_message = r#"{"type":"chat", "te"#;

        let json_data: Result<IncomingMessage, serde_json::Error> =
            serde_json::from_str(own_message);

        assert!(json_data.is_err());
    }

    #[test]
    fn outgoing_chat_matches_protocol() {
        let outgoing_message = OutgoingMessage::Chat {
            sender_id: 1,
            sender_name: "Alice".to_string(),
            text: "Hallo".to_string(),
            room: "welcome".to_string(),
            timestamp: 123,
        };

        let json_data: String =
            serde_json::to_string(&outgoing_message).expect("Could not turn...");

        assert!(json_data.contains(r#""sender_name":"Alice""#));
        assert!(json_data.contains(r#""type":"chat""#));
    }

    #[test]
    fn test_error() {
        let outgoing_message = OutgoingMessage::Error {
            code: ErrorCode::NotLoggedIn,
            message: ("Bitte zuerst anmelden.".to_string()),
        };

        let json_data =
            serde_json::to_string(&outgoing_message).expect("Could not read from JSON.");

        assert!(json_data.contains(r#""type":"error""#));
        assert!(json_data.contains(r#""code":"not_logged_in""#));
    }
}
