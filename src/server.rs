use crate::chat_room::ChatRoom;
use actix::prelude::*;
use std::collections::HashMap;

/// Nachricht vom Server an einen Client. Wird an alle Mitglieder eines Raums verteilt.
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct ServerMessage {
    pub sender_id: u64,
    pub sender_name: String,
    pub text: String,
    pub room: String,
    pub timestamp: u64,
}

/// Zentraler Actor des Chats. Verwaltet alle Clients und Räume. Da Actors Nachrichten nacheinander verarbeiten, braucht der Zustand keine Locks.
pub struct ChatServer {
    ///Speichert den Raumname und die dazugehörigen Client ID's
    pub rooms: HashMap<String, ChatRoom>,
    ///Speichern von Empfängeradressen unter einer Client ID
    pub clients: HashMap<u64, Client>,
    pub next_client_id: u64,
}

/// Ein verbundener Client.
pub struct Client {
    pub name: String,
    pub addr: Recipient<ServerMessage>,
}

/// Ein Client möchte eine Nachricht in einen Raum schreiben.
#[derive(Message)]
#[rtype(result = "()")]
pub struct ClientMessage {
    pub id: u64,
    pub msg: String,
    pub room: String,
}

/// Ein Client tritt einem Raum bei. Existiert der Raum nicht, wird er angelegt.
#[derive(Message)]
#[rtype(result = "()")]
pub struct Join {
    pub room: String,
    pub client_id: u64,
}

/// Ein Client verlässt einen Raum.
#[derive(Message)]
#[rtype(result = "()")]
pub struct Leave {
    pub room: String,
    pub client_id: u64,
}

/// Registriert einen neuen Client beim Server. Antwortet mit der vergebenen Client-ID.
#[derive(Message)]
#[rtype(result = "u64")]
pub struct Connect {
    pub client_addr: Recipient<ServerMessage>,
    pub name: String,
}

/// Ein Client hat die Verbindung beendet und wird entfernt.
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub client_id: u64,
}

impl Actor for ChatServer {
    type Context = Context<Self>;
}

impl ChatServer {
    /// Erstellt einen leeren Server mit dem Standardraum "welcome".
    pub fn new() -> ChatServer {
        let mut rooms = HashMap::new();
        rooms.insert("welcome".to_string(), ChatRoom::new());

        ChatServer {
            rooms,
            clients: HashMap::new(),
            next_client_id: 1,
        }
    }
}

impl ChatServer {
    /// Schickt eine Kopie der Nachricht an alle Mitglieder des Raums. Unbekannte IDs werden übersprungen.
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

impl Handler<Connect> for ChatServer {
    type Result = u64;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> u64 {
        let client_id = self.next_client_id;
        let new_client = Client {
            name: msg.name,
            addr: msg.client_addr,
        };
        self.clients.insert(client_id, new_client);
        self.next_client_id += 1;

        println!("Client verbunden mit id: {}", client_id);

        // Jeder neue Client landet automatisch in "welcome", damit er sofort chatten kann.
        self.rooms
            .entry("welcome".to_string())
            .or_insert_with(ChatRoom::new)
            .join(client_id);

        client_id
    }
}

/// Entfernt den Client vom Server und aus allen Räumen
impl Handler<Disconnect> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        self.clients.remove(&msg.client_id);

        for (room_name, room) in self.rooms.iter_mut() {
            if room.leave(msg.client_id) {
                println!(
                    "Client {} hat den Raum {} verlassen",
                    msg.client_id, room_name
                );
            }
        }
    }
}

impl Handler<Join> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Join, _: &mut Context<Self>) {
        let room_name = msg.room.clone();
        let joined = self
            .rooms
            .entry(msg.room)
            .or_insert_with(ChatRoom::new)
            .join(msg.client_id);

        if joined {
            println!(
                "Client {} ist dem Raum {} beigetreten",
                msg.client_id, room_name
            );
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

impl Handler<ClientMessage> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: ClientMessage, _: &mut Context<Self>) {
        let Some(client) = self.clients.get(&msg.id) else {
            return;
        };
        let Some(room) = self.rooms.get(&msg.room) else {
            return;
        };

        if !room.get_members().contains(&msg.id) {
            return;
        }

        if msg.msg.trim().is_empty() {
            return;
        }

        if msg.msg.chars().count() > 1000 {
            return;
        }

        let sender_name = client.name.clone();

        let server_message = ServerMessage {
            sender_id: msg.id,
            sender_name,
            text: msg.msg,
            room: msg.room,
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
        };

        self.send_message(&server_message.room, &server_message);
    }
}

#[cfg(test)]
mod tests {

    use std::{
        sync::mpsc::{Receiver, Sender, channel},
        time::Duration,
    };

    use super::*;
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

    async fn connect(
        server: &Addr<ChatServer>,
        client_name: &str,
    ) -> (u64, Receiver<ServerMessage>) {
        let (sender, receiver) = channel();

        let recording_client = RecordingClient { sender_end: sender }.start();

        let id = server
            .send(Connect {
                client_addr: recording_client.recipient(),
                name: client_name.to_string(),
            })
            .await
            .expect("Unable to connect to server.");

        (id, receiver)
    }

    async fn write(server: &Addr<ChatServer>, client_id: u64, sent_to_room: &str, sent_text: &str) {
        server
            .send(ClientMessage {
                id: client_id,
                msg: sent_text.to_string(),
                room: sent_to_room.to_string(),
            })
            .await
            .expect("Could not send message.");
    }

    async fn received(receiver: &Receiver<ServerMessage>) -> Vec<ServerMessage> {
        actix::clock::sleep(Duration::from_millis(50)).await;
        receiver.try_iter().collect()
    }

    async fn joined(server: &Addr<ChatServer>, id: u64, joined_room: &str) {
        server
            .send(Join {
                room: joined_room.to_string(),
                client_id: id,
            })
            .await
            .expect("Could not join room.");
    }

    async fn disconnect(server: &Addr<ChatServer>, id: u64) {
        server
            .send(Disconnect { client_id: id })
            .await
            .expect("Client was unable to disconnect from server");
    }

    #[actix::test]
    async fn ids_are_assigned_in_order() {
        let server = ChatServer::new().start();

        let (id_alice, _receiver_alice) = connect(&server, "Alice").await;
        let (id_bob, _receiver_bob) = connect(&server, "Bob").await;

        assert_eq!(id_alice, 1);
        assert_eq!(id_bob, 2);
    }

    #[actix::test]
    async fn sender_receives_own_message() {
        let server = ChatServer::new().start();

        let (id, receiver) = connect(&server, "Alice").await;

        write(&server, id, "welcome", "Hallo").await;

        let received_messages = received(&receiver).await;

        assert_eq!(received_messages.len(), 1);
        assert_eq!(received_messages[0].text, "Hallo");
    }

    #[actix::test]
    async fn everybody_gets_message() {
        let server = ChatServer::new().start();

        let (id_alice, receiver_alice) = connect(&server, "Alice").await;
        let (_id_bob, receiver_bob) = connect(&server, "Bob").await;

        write(&server, id_alice, "welcome", "Hallo").await;

        let messages_alice = received(&receiver_alice).await;
        let messages_bob = received(&receiver_bob).await;

        assert_eq!(messages_alice.len(), 1);
        assert_eq!(messages_bob.len(), 1);

        assert_eq!(messages_alice[0].text, "Hallo");
        assert_eq!(messages_bob[0].text, "Hallo");
    }

    #[actix::test]
    async fn all_fields_correct() {
        let server = ChatServer::new().start();

        let (id_alice, receiver_alice) = connect(&server, "Alice").await;

        write(&server, id_alice, "welcome", "Hallo").await;

        let messages_alice = received(&receiver_alice).await;

        assert!(!messages_alice.is_empty(), "Error no message arrived.");

        assert_eq!(messages_alice[0].sender_id, id_alice);
        assert_eq!(messages_alice[0].sender_name, "Alice");
        assert_eq!(messages_alice[0].text, "Hallo");
        assert_eq!(messages_alice[0].room, "welcome");
        assert!(messages_alice[0].timestamp > 0);
    }

    #[actix::test]
    async fn empty_message_is_ignored() {
        let server = ChatServer::new().start();

        let (id_alice, receiver_alice) = connect(&server, "Alice").await;

        write(&server, id_alice, "welcome", "").await;
        write(&server, id_alice, "welcome", "   ").await;

        let messages_alice = received(&receiver_alice).await;

        assert_eq!(messages_alice.len(), 0);
    }

    #[actix::test]
    async fn message_ignored_if_too_long() {
        let server = ChatServer::new().start();

        let (id_alice, receiver_alice) = connect(&server, "Alice").await;

        write(&server, id_alice, "welcome", &("aba".repeat(340))).await;

        let messages_alice = received(&receiver_alice).await;

        assert_eq!(messages_alice.len(), 0);
    }

    #[actix::test]
    async fn thousand_emojis_accepted() {
        let server = ChatServer::new().start();

        let (id_alice, receiver_alice) = connect(&server, "Alice").await;

        write(&server, id_alice, "welcome", &("😀".repeat(1000))).await;

        let messages_alice = received(&receiver_alice).await;

        assert_eq!(messages_alice.len(), 1);
    }

    #[actix::test]
    async fn unknown_sender_is_ignored() {
        let server = ChatServer::new().start();

        let (_id_alice, receiver_alice) = connect(&server, "Alice").await;

        write(&server, 999, "welcome", "Hallo").await;

        let messages_alice = received(&receiver_alice).await;

        assert_eq!(messages_alice.len(), 0);
    }

    #[actix::test]
    async fn write_to_non_existent_room() {
        let server = ChatServer::new().start();

        let (id_alice, receiver_alice) = connect(&server, "Alice").await;

        write(&server, id_alice, "unknown_room", "Hallo").await;

        let messages_alice = received(&receiver_alice).await;

        assert_eq!(messages_alice.len(), 0);
    }

    #[actix::test]
    async fn sender_outside_the_room() {
        let server = ChatServer::new().start();

        let (id_alice, receiver_alice) = connect(&server, "Alice").await;
        let (id_bob, _receiver_bob) = connect(&server, "Bob").await;

        joined(&server, id_alice, "other_room").await;
        write(&server, id_bob, "other_room", "Hallo").await;

        let messages_alice = received(&receiver_alice).await;

        assert_eq!(messages_alice.len(), 0);
    }

    #[actix::test]
    async fn disconnected_client_receives_nothing() {
        let server = ChatServer::new().start();

        let (id_alice, receiver_alice) = connect(&server, "Alice").await;
        let (id_bob, receiver_bob) = connect(&server, "Bob").await;

        disconnect(&server, id_bob).await;

        write(&server, id_alice, "welcome", "Hallo").await;

        let messages_alice = received(&receiver_alice).await;
        let messages_bob = received(&receiver_bob).await;

        assert_eq!(messages_alice.len(), 1);
        assert_eq!(messages_bob.len(), 0);
    }

    #[actix::test]
    async fn disconnected_client_cannot_write() {
        let server = ChatServer::new().start();

        let (_id_alice, receiver_alice) = connect(&server, "Alice").await;
        let (id_bob, _receiver_bob) = connect(&server, "Bob").await;

        disconnect(&server, id_bob).await;

        write(&server, id_bob, "welcome", "Hallo").await;

        let messages_alice = received(&receiver_alice).await;

        assert_eq!(messages_alice.len(), 0);
    }
}
