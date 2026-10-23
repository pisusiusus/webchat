use actix::{Actor, ActorContext, StreamHandler};
use actix_web_actors::ws::{self, WebsocketContext};

/// Eine WebSocket-Verbindung zu einem Browser.
/// Pro verbundemen Browser gibt es eine Session.
pub struct ChatSession;

impl Actor for ChatSession {
    type Context = WebsocketContext<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        println!("Session gestartet.");
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        println!("Session beendet.");
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ChatSession {
    fn handle(
        &mut self,
        item: Result<ws::Message, ws::ProtocolError>,
        ctx: &mut Self::Context,
    ) {
        match item {
            Ok(ws::Message::Text(text)) => ctx.text(text),
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop()
            }
            Err(_) => ctx.stop(),
            Ok(ws::Message::Ping(bytes)) => ctx.pong(&bytes),
            Ok(ws::Message::Pong(_)) => {}, // TODO
            Ok(ws::Message::Binary(_)) => {}, // TODO
            _ => {}
        }
    }
}
