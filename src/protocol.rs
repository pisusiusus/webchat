//! Die JSON-Nachrichten zwischen Browser und Server.
//! Varianten- und Feldnamen sind gleichzeitig die JSON-Namen.
//! Umbennenen ändert das Protokoll.

use serde::{Deserialize, Serialize};

use crate::chat_room::ChatMessage;

/// Nachrichten vom Browser an den Server.
/// `tag = "type"`: Die Variante steht im JSON-Feld `type`,
/// z. B. wird `Login` zu `"type":"login", ...`.
#[derive(Deserialize, Debug, PartialEq)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum IncomingMessage {
    /// Muss die erste Nachricht einer Verbindung sein.
    Login {
        name: String,
    },

    Chat {
        text: String,
        room: String
    },

    History {
        room: String
    },

    GetRooms,

    CreateRoom {
        room: String
    },

    JoinRoom {
        room: String
    },

    LeaveRoom {
        room: String
    }
}

/// Nachrichten vom Server an den Browser.
#[derive(Serialize, Debug, PartialEq)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum OutgoingMessage {
    /// Antwort auf ein erfolgreiches Login.
    /// Mit `id` erkennt das Frontend später seine eigenen Nachrichten.
    Welcome { id: u64, name: String },

    /// Geht an alle im Raum, auch an den Absender.
    Chat {
        sender_id: u64,
        sender_name: String,
        text: String,
        room: String,
        timestamp: u64,
    },

    /// Die Verbindung bleibt nach einem Fehler offen.
    Error { code: ErrorCode, message: String },

    RoomList {
        rooms: Vec<String>,
    },

    History {
        room: String,
        messages: Vec<ChatMessage>
    },

    RoomJoined {
        room: String
    },

    RoomLeft {
        room: String
    }
}

/// Maschinenlesbare Fehlercodes. Bedeutungen stehen unter: `docs/protocol.md`
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidJson,
    NotLoggedIn,
    AlreadyLoggedIn,
    InvalidName,
    RoomError,
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn login_json_becomes_login_message() {
        let login_message = IncomingMessage::Login {
            name: "Alice".to_string(),
        };

        let own_message = r#"{"type":"login", "name":"Alice"}"#;

        let json_data: IncomingMessage =
            serde_json::from_str(own_message).expect("JSON konnte nicht gelesen werden.");

        assert_eq!(login_message, json_data);
    }

    #[test]
    fn chat_json_becomes_chat_message() {
        let chat_message = IncomingMessage::Chat {
            text: "Hallo".to_string(),
            room: "welcome".to_string()
        };

        let own_message = r#"{"type":"chat", "text":"Hallo", "room":"welcome"}"#;

        let json_data: IncomingMessage =
            serde_json::from_str(own_message).expect("JSON  konnte nicht gelesen werden.");

        assert_eq!(chat_message, json_data);
    }

    #[test]
    fn unknown_type_is_rejected() {
        let own_message = r#"{"type":"unknown", "text":"Hallo"}"#;

        let json_data: Result<IncomingMessage, serde_json::Error> =
            serde_json::from_str(own_message);

        assert!(json_data.is_err());
    }

    #[test]
    fn broken_json_is_rejected() {
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

        let json_data: String = serde_json::to_string(&outgoing_message)
            .expect("Nachricht konnte nicht in JSON umgewandelt werden.");

        assert!(json_data.contains(r#""sender_name":"Alice""#));
        assert!(json_data.contains(r#""type":"chat""#));
    }

    #[test]
    fn error_code_is_written_in_snake_case() {
        let outgoing_message = OutgoingMessage::Error {
            code: ErrorCode::NotLoggedIn,
            message: "Bitte zuerst anmelden.".to_string(),
        };

        let json_data = serde_json::to_string(&outgoing_message)
            .expect("Nachricht konnte nicht in JSON umgewandelt werden.");

        assert!(json_data.contains(r#""type":"error""#));
        assert!(json_data.contains(r#""code":"not_logged_in""#));
    }

    #[test]
    fn room_requests_match_protocol() {
        let messages = [
            (r#"{"type":"create_room","room":"general"}"#, IncomingMessage::CreateRoom { room: "general".to_string() }),
            (r#"{"type":"join_room","room":"general"}"#, IncomingMessage::JoinRoom { room: "general".to_string() }),
            (r#"{"type":"leave_room","room":"general"}"#, IncomingMessage::LeaveRoom { room: "general".to_string() }),
            (r#"{"type":"get_rooms"}"#, IncomingMessage::GetRooms),
        ];

        for (json, expected) in messages {
            let message: IncomingMessage = serde_json::from_str(json)
                .expect("JSON konnte nicht gelesen werden.");
            assert_eq!(message, expected);
        }
    }
}
