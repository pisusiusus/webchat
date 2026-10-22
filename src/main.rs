mod chat_room;
mod server;

use actix::prelude::*;
use server::{Connect, Join, ServerMessage};

struct TestClient;

impl Actor for TestClient {
    type Context = Context<Self>;
}

impl Handler<ServerMessage> for TestClient {
    type Result = ();

    fn handle(&mut self, msg: ServerMessage, _: &mut Context<Self>) {
        println!("Nachricht empfangen: {} - {}", msg.sender_name, msg.text);
    }
}

#[actix::main]
async fn main() {
    let server = server::ChatServer::new().start();

    let client = TestClient.start();

    let id = server
        .send(Connect {
            client_addr: client.recipient(),
            name: "TestClient".to_string(),
        })
        .await
        .expect("Connection failed");

    server
        .send(Join {
            room: "test".to_string(),
            client_id: id,
        })
        .await
        .expect("Join failed");
}
