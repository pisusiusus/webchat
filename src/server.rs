use crate::chat_room::ChatRoom;
use actix::prelude::*;
use std::collections::HashMap;

//Message
#[derive(Message)]
#[rtype(result = "()")]
pub struct ServerMessage(pub String);

//ChatServer Actor
pub struct ChatServer {
    //Speichert den Raumname und die dazugehörigen Client ID's
    pub rooms: HashMap<String, ChatRoom>,
    //Speichern von Empfängeradressen unter einer Client ID
    pub clients: HashMap<u64, Client>,
    pub next_client_id: u64,
}

pub struct Client {
    pub name: String,
    pub addr: Recipient<ServerMessage>,
}

#[derive(Message)]
#[rtype(result = "()")]
//Message from Client -> Server
pub struct ClientMessage {
    pub id: u64,
    pub msg: String,
    pub room: String,
}

#[derive(Message)]
#[rtype(result = "()")]
//Client tritt Room bei
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
//Client Connect -> Registrieren als Client
pub struct Connect {
    pub client_addr: Recipient<ServerMessage>,
    pub name: String,
}

#[derive(Message)]
#[rtype(result = "()")]
//Client Disconnect
pub struct Disconnect {
    pub client_id: u64,
}

impl Actor for ChatServer {
    type Context = Context<Self>;
}

//chatServer Konstruktor
impl ChatServer {
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
    //Nachricht an alle Clients in einem Raum senden.
    pub fn send_message(&self, room: &str, message: &str) {
        if let Some(chat_room) = self.rooms.get(room) {
            for client_id in chat_room.get_members() {
                if let Some(client) = self.clients.get(&client_id) {
                    client.addr.do_send(ServerMessage(message.to_string()));
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

        //standardmäßig mit 'welcome' verbinden
        self.rooms
            .entry("welcome".to_string())
            .or_insert_with(ChatRoom::new)
            .join(client_id);

        client_id
    }
}

impl Handler<Disconnect> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        self.clients.remove(&msg.client_id);
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
