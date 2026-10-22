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
            rooms: rooms,
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
                if let Some(client) = self.clients.get(&client_id) {
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

// TODO: Client auch aus allen Räumen entfernen.
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
        let Some(room) = self.rooms.get(&msg.room) else  {
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
