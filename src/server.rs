//! Chat state and actor messages. Independent of JSON and WebSockets.
use crate::chat_room::{ChatMessage, ChatRoom};
use actix::prelude::*;
use std::collections::HashMap;

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct ServerMessage {
    pub sender_id: u64,
    pub sender_name: String,
    pub text: String,
    pub room: String,
    pub timestamp: u64,
}

pub struct ChatServer {
    pub rooms: HashMap<String, ChatRoom>,
    pub clients: HashMap<u64, Client>,
    pub next_client_id: u64,
}

pub struct Client {
    pub name: String,
    pub addr: Recipient<ServerMessage>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ClientMessage {
    pub id: u64,
    pub msg: String,
    pub room: String,
}

#[derive(Message)]
#[rtype(result = "Vec<String>")]
pub struct GetRooms;

#[derive(Message)]
#[rtype(result = "Option<Vec<ChatMessage>>")]
pub struct GetHistory {
    pub room: String,
    pub client_id: u64,
}

/// Legacy internal API: joins a room, creating it if necessary.
/// Browser requests use the validated ManageRoom API instead.
#[derive(Message)]
#[rtype(result = "()")]
pub struct Join {
    pub room: String,
    pub client_id: u64,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Leave {
    pub room: String,
    pub client_id: u64,
}

#[derive(Message)]
#[rtype(result = "u64")]
pub struct Connect {
    pub client_addr: Recipient<ServerMessage>,
    pub name: String,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub client_id: u64,
}

pub enum RoomAction {
    Create,
    Join,
    Leave,
}

#[derive(Message)]
#[rtype(result = "Result<String, String>")]
pub struct ManageRoom {
    pub client_id: u64,
    pub room: String,
    pub action: RoomAction,
}

pub struct RoomState {
    pub rooms: Vec<String>,
    pub joined_rooms: Vec<String>,
}

#[derive(Message)]
#[rtype(result = "RoomState")]
pub struct GetRoomState {
    pub client_id: u64,
}

impl Actor for ChatServer {
    type Context = Context<Self>;
}

impl ChatServer {
    pub fn new() -> Self {
        let mut rooms = HashMap::new();
        rooms.insert("welcome".to_string(), ChatRoom::new());
        Self {
            rooms,
            clients: HashMap::new(),
            next_client_id: 1,
        }
    }

    pub fn send_message(&self, room: &str, message: &ServerMessage) {
        if let Some(chat_room) = self.rooms.get(room) {
            for client_id in chat_room.get_members() {
                if let Some(client) = self.clients.get(client_id) {
                    client.addr.do_send(message.clone());
                }
            }
        }
    }
}

impl Handler<GetRooms> for ChatServer {
    type Result = Vec<String>;

    fn handle(&mut self, _: GetRooms, _: &mut Context<Self>) -> Self::Result {
        let mut rooms: Vec<String> = self.rooms.keys().cloned().collect();
        rooms.sort();
        rooms
    }
}

impl Handler<GetRoomState> for ChatServer {
    type Result = MessageResult<GetRoomState>;

    fn handle(&mut self, msg: GetRoomState, _: &mut Context<Self>) -> Self::Result {
        let mut rooms: Vec<String> = self.rooms.keys().cloned().collect();
        let mut joined_rooms: Vec<String> = self
            .rooms
            .iter()
            .filter(|(_, room)| room.get_members().contains(&msg.client_id))
            .map(|(name, _)| name.clone())
            .collect();
        rooms.sort();
        joined_rooms.sort();
        MessageResult(RoomState {
            rooms,
            joined_rooms,
        })
    }
}

impl Handler<GetHistory> for ChatServer {
    type Result = MessageResult<GetHistory>;

    fn handle(&mut self, msg: GetHistory, _: &mut Context<Self>) -> Self::Result {
        let history = if self.clients.contains_key(&msg.client_id) {
            self.rooms.get(&msg.room).and_then(|room| {
                if room.get_members().contains(&msg.client_id) {
                    Some(room.messages.clone())
                } else {
                    None
                }
            })
        } else {
            None
        };
        MessageResult(history)
    }
}

impl Handler<Connect> for ChatServer {
    type Result = u64;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        let id = self.next_client_id;
        self.next_client_id += 1;
        self.clients.insert(
            id,
            Client {
                name: msg.name,
                addr: msg.client_addr,
            },
        );
        self.rooms
            .entry("welcome".to_string())
            .or_insert_with(ChatRoom::new)
            .join(id);
        println!("Client connected: {id}");
        id
    }
}

impl Handler<Disconnect> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        self.clients.remove(&msg.client_id);
        for room in self.rooms.values_mut() {
            room.leave(msg.client_id);
        }
    }
}

impl Handler<Join> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Join, _: &mut Context<Self>) {
        if self.clients.contains_key(&msg.client_id) {
            self.rooms
                .entry(msg.room)
                .or_insert_with(ChatRoom::new)
                .join(msg.client_id);
        }
    }
}

impl Handler<Leave> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Leave, _: &mut Context<Self>) {
        if let Some(room) = self.rooms.get_mut(&msg.room) {
            room.leave(msg.client_id);
        }
    }
}

impl Handler<ManageRoom> for ChatServer {
    type Result = Result<String, String>;

    fn handle(&mut self, msg: ManageRoom, _: &mut Context<Self>) -> Self::Result {
        if !self.clients.contains_key(&msg.client_id) {
            return Err("Client is not logged in.".to_string());
        }

        let name = msg.room.trim().to_string();
        if name.is_empty()
            || name.chars().count() > 40
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err("Room names must contain 1-40 letters, numbers, '-' or '_'.".into());
        }

        match msg.action {
            RoomAction::Create => {
                if self.rooms.contains_key(&name) {
                    return Err("Room already exists.".into());
                }
                let mut room = ChatRoom::new();
                room.join(msg.client_id);
                self.rooms.insert(name.clone(), room);
            }
            RoomAction::Join => {
                let room = self
                    .rooms
                    .get_mut(&name)
                    .ok_or_else(|| "Room does not exist.".to_string())?;
                room.join(msg.client_id);
            }
            RoomAction::Leave => {
                let room = self
                    .rooms
                    .get_mut(&name)
                    .ok_or_else(|| "Room does not exist.".to_string())?;
                room.leave(msg.client_id);
            }
        }
        Ok(name)
    }
}

impl Handler<ClientMessage> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: ClientMessage, _: &mut Context<Self>) {
        let Some(client) = self.clients.get(&msg.id) else {
            return;
        };
        let Some(room) = self.rooms.get_mut(&msg.room) else {
            return;
        };
        if !room.get_members().contains(&msg.id)
            || msg.msg.trim().is_empty()
            || msg.msg.chars().count() > 1000
        {
            return;
        }

        let message = ServerMessage {
            sender_id: msg.id,
            sender_name: client.name.clone(),
            text: msg.msg,
            room: msg.room,
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
        };
        room.add_message(ChatMessage {
            sender_id: message.sender_id,
            sender_name: message.sender_name.clone(),
            text: message.text.clone(),
            timestamp: message.timestamp,
        });
        self.send_message(&message.room, &message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::mpsc::{Receiver, Sender, channel},
        time::Duration,
    };

    struct RecordingClient {
        sender_end: Sender<ServerMessage>,
    }
    impl Actor for RecordingClient {
        type Context = Context<Self>;
    }
    impl Handler<ServerMessage> for RecordingClient {
        type Result = ();
        fn handle(&mut self, msg: ServerMessage, _: &mut Context<Self>) {
            let _ = self.sender_end.send(msg);
        }
    }

    async fn connect(server: &Addr<ChatServer>, name: &str) -> (u64, Receiver<ServerMessage>) {
        let (sender, receiver) = channel();
        let client = RecordingClient { sender_end: sender }.start();
        let id = server
            .send(Connect {
                client_addr: client.recipient(),
                name: name.to_string(),
            })
            .await
            .unwrap();
        (id, receiver)
    }
    async fn write(server: &Addr<ChatServer>, id: u64, room: &str, text: &str) {
        server
            .send(ClientMessage {
                id,
                room: room.into(),
                msg: text.into(),
            })
            .await
            .unwrap();
    }
    async fn received(receiver: &Receiver<ServerMessage>) -> Vec<ServerMessage> {
        actix::clock::sleep(Duration::from_millis(50)).await;
        receiver.try_iter().collect()
    }
    async fn joined(server: &Addr<ChatServer>, id: u64, room: &str) {
        server
            .send(Join {
                room: room.into(),
                client_id: id,
            })
            .await
            .unwrap();
    }
    async fn disconnect(server: &Addr<ChatServer>, id: u64) {
        server.send(Disconnect { client_id: id }).await.unwrap();
    }
    async fn manage(
        server: &Addr<ChatServer>,
        id: u64,
        room: &str,
        action: RoomAction,
    ) -> Result<String, String> {
        server
            .send(ManageRoom {
                client_id: id,
                room: room.into(),
                action,
            })
            .await
            .unwrap()
    }

    #[actix::test]
    async fn ids_are_assigned_in_order() {
        let s = ChatServer::new().start();
        let (a, _) = connect(&s, "Alice").await;
        let (b, _) = connect(&s, "Bob").await;
        assert_eq!((a, b), (1, 2));
    }

    #[actix::test]
    async fn sender_receives_own_message() {
        let s = ChatServer::new().start();
        let (a, inbox) = connect(&s, "Alice").await;
        write(&s, a, "welcome", "Hallo").await;
        let messages = received(&inbox).await;
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].text, "Hallo");
    }

    #[actix::test]
    async fn all_room_members_receive_message() {
        let s = ChatServer::new().start();
        let (a, ai) = connect(&s, "Alice").await;
        let (_, bi) = connect(&s, "Bob").await;
        write(&s, a, "welcome", "Hallo").await;
        let am = received(&ai).await;
        let bm = received(&bi).await;
        assert_eq!(am.len(), 1);
        assert_eq!(bm.len(), 1);
        assert_eq!(am[0].text, "Hallo");
        assert_eq!(bm[0].text, "Hallo");
    }

    #[actix::test]
    async fn message_contains_correct_fields() {
        let s = ChatServer::new().start();
        let (a, inbox) = connect(&s, "Alice").await;
        write(&s, a, "welcome", "Hallo").await;
        let messages = received(&inbox).await;
        assert_eq!(messages.len(), 1);
        let m = &messages[0];
        assert_eq!(m.sender_id, a);
        assert_eq!(m.sender_name, "Alice");
        assert_eq!(m.text, "Hallo");
        assert_eq!(m.room, "welcome");
        assert!(m.timestamp > 0);
    }

    #[actix::test]
    async fn empty_message_is_ignored() {
        let s = ChatServer::new().start();
        let (a, inbox) = connect(&s, "Alice").await;
        write(&s, a, "welcome", "").await;
        write(&s, a, "welcome", "   ").await;
        assert!(received(&inbox).await.is_empty());
    }

    #[actix::test]
    async fn too_long_message_is_ignored() {
        let s = ChatServer::new().start();
        let (a, inbox) = connect(&s, "Alice").await;
        write(&s, a, "welcome", &"aba".repeat(340)).await;
        assert!(received(&inbox).await.is_empty());
    }

    #[actix::test]
    async fn length_limit_counts_characters_not_bytes() {
        let s = ChatServer::new().start();
        let (a, inbox) = connect(&s, "Alice").await;
        write(&s, a, "welcome", &"😀".repeat(1000)).await;
        assert_eq!(received(&inbox).await.len(), 1);
    }

    #[actix::test]
    async fn unknown_sender_is_ignored() {
        let s = ChatServer::new().start();
        let (_, inbox) = connect(&s, "Alice").await;
        write(&s, 999, "welcome", "Hallo").await;
        assert!(received(&inbox).await.is_empty());
    }

    #[actix::test]
    async fn message_to_unknown_room_is_ignored() {
        let s = ChatServer::new().start();
        let (a, inbox) = connect(&s, "Alice").await;
        write(&s, a, "unknown_room", "Hallo").await;
        assert!(received(&inbox).await.is_empty());
    }

    #[actix::test]
    async fn non_member_cannot_write_to_room() {
        let s = ChatServer::new().start();
        let (a, inbox) = connect(&s, "Alice").await;
        let (b, _) = connect(&s, "Bob").await;
        joined(&s, a, "other_room").await;
        write(&s, b, "other_room", "Hallo").await;
        assert!(received(&inbox).await.is_empty());
    }

    #[actix::test]
    async fn disconnected_client_receives_nothing() {
        let s = ChatServer::new().start();
        let (a, ai) = connect(&s, "Alice").await;
        let (b, bi) = connect(&s, "Bob").await;
        disconnect(&s, b).await;
        write(&s, a, "welcome", "Hallo").await;
        assert_eq!(received(&ai).await.len(), 1);
        assert!(received(&bi).await.is_empty());
    }

    #[actix::test]
    async fn disconnected_client_cannot_write() {
        let s = ChatServer::new().start();
        let (_, ai) = connect(&s, "Alice").await;
        let (b, _) = connect(&s, "Bob").await;
        disconnect(&s, b).await;
        write(&s, b, "welcome", "Hallo").await;
        assert!(received(&ai).await.is_empty());
    }

    #[actix::test]
    async fn room_creation_join_leave_and_history_access() {
        let s = ChatServer::new().start();
        let (a, ai) = connect(&s, "Alice").await;
        let (b, bi) = connect(&s, "Bob").await;
        assert_eq!(
            manage(&s, a, "general", RoomAction::Create).await.unwrap(),
            "general"
        );
        let state = s.send(GetRoomState { client_id: a }).await.unwrap();
        assert!(state.rooms.contains(&"general".into()));
        assert!(state.joined_rooms.contains(&"general".into()));
        assert!(manage(&s, b, "general", RoomAction::Create).await.is_err());
        assert!(
            s.send(GetHistory {
                room: "general".into(),
                client_id: b
            })
            .await
            .unwrap()
            .is_none()
        );
        manage(&s, b, "general", RoomAction::Join).await.unwrap();
        write(&s, a, "general", "Hello").await;
        assert_eq!(received(&ai).await.len(), 1);
        assert_eq!(received(&bi).await.len(), 1);
        let history = s
            .send(GetHistory {
                room: "general".into(),
                client_id: b,
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].text, "Hello");
        manage(&s, b, "general", RoomAction::Leave).await.unwrap();
        write(&s, a, "general", "After leave").await;
        assert_eq!(received(&ai).await.len(), 1);
        assert!(received(&bi).await.is_empty());
        write(&s, b, "general", "Not allowed").await;
        assert!(received(&ai).await.is_empty());
        assert!(
            s.send(GetHistory {
                room: "general".into(),
                client_id: b
            })
            .await
            .unwrap()
            .is_none()
        );
        assert!(manage(&s, b, "missing", RoomAction::Join).await.is_err());
        assert!(
            manage(&s, a, "bad room!", RoomAction::Create)
                .await
                .is_err()
        );
        assert!(
            manage(&s, 999, "another", RoomAction::Create)
                .await
                .is_err()
        );
    }
}
