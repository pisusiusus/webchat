mod server;
mod chat_room;

use actix::prelude::*;
use server::{ChatServer, Join, Connect, Message};

struct TestClient;

impl Actor for TestClient {
    type Context = Context<Self>;
}

impl Handler<Message> for TestClient {
    type Result = ();

    fn handle(&mut self, msg: Message, _: &mut Context<Self>) {
        println!("Nachricht empfangen: {}", msg.0);
    }
}


#[actix::main]
async fn main(){

    let server = server::ChatServer::new().start();

    let client = TestClient.start();

    let id = server.send(Connect {
        client_addr: client.recipient(),
        name: "TestClient".to_string()
    })
    .await
    .expect("Connection failed");

    server.send(Join {
        room: "test".to_string(),
        client_id: id
    })
    .await
    .expect("Join failed");

}