//! JSON messages exchanged between the browser and server.
use crate::chat_room::ChatMessage;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IncomingMessage {
    Login { name: String },
    Chat { text: String, room: String },
    History { room: String },
    CreateRoom { room: String },
    JoinRoom { room: String },
    LeaveRoom { room: String },
    ListRooms,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
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
    RoomList {
        rooms: Vec<String>,
        joined_rooms: Vec<String>,
    },
    History {
        room: String,
        messages: Vec<ChatMessage>,
    },
    RoomJoined {
        room: String,
    },
    RoomLeft {
        room: String,
    },
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidJson,
    NotLoggedIn,
    AlreadyLoggedIn,
    InvalidName,
    RoomOperationFailed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_json_becomes_login_message() {
        let parsed: IncomingMessage =
            serde_json::from_str(r#"{"type":"login","name":"Alice"}"#).unwrap();
        assert_eq!(
            parsed,
            IncomingMessage::Login {
                name: "Alice".into()
            }
        );
    }

    #[test]
    fn chat_json_becomes_chat_message() {
        let parsed: IncomingMessage =
            serde_json::from_str(r#"{"type":"chat","text":"Hallo","room":"welcome"}"#).unwrap();
        assert_eq!(
            parsed,
            IncomingMessage::Chat {
                text: "Hallo".into(),
                room: "welcome".into(),
            }
        );
    }

    #[test]
    fn unknown_type_is_rejected() {
        assert!(
            serde_json::from_str::<IncomingMessage>(r#"{"type":"unknown","text":"Hallo"}"#,)
                .is_err()
        );
    }

    #[test]
    fn broken_json_is_rejected() {
        assert!(serde_json::from_str::<IncomingMessage>(r#"{"type":"chat","te"#,).is_err());
    }

    #[test]
    fn outgoing_chat_matches_protocol() {
        let json = serde_json::to_string(&OutgoingMessage::Chat {
            sender_id: 1,
            sender_name: "Alice".into(),
            text: "Hallo".into(),
            room: "welcome".into(),
            timestamp: 123,
        })
        .unwrap();
        assert!(json.contains(r#""sender_name":"Alice""#));
        assert!(json.contains(r#""type":"chat""#));
    }

    #[test]
    fn error_code_is_written_in_snake_case() {
        let json = serde_json::to_string(&OutgoingMessage::Error {
            code: ErrorCode::NotLoggedIn,
            message: "Please log in first.".into(),
        })
        .unwrap();
        assert!(json.contains(r#""type":"error""#));
        assert!(json.contains(r#""code":"not_logged_in""#));
    }

    #[test]
    fn room_commands_are_deserialized() {
        let cases = [
            (
                r#"{"type":"create_room","room":"general"}"#,
                IncomingMessage::CreateRoom {
                    room: "general".into(),
                },
            ),
            (
                r#"{"type":"join_room","room":"general"}"#,
                IncomingMessage::JoinRoom {
                    room: "general".into(),
                },
            ),
            (
                r#"{"type":"leave_room","room":"general"}"#,
                IncomingMessage::LeaveRoom {
                    room: "general".into(),
                },
            ),
            (r#"{"type":"list_rooms"}"#, IncomingMessage::ListRooms),
        ];
        for (json, expected) in cases {
            assert_eq!(
                serde_json::from_str::<IncomingMessage>(json).unwrap(),
                expected
            );
        }
    }
}
