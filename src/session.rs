use actix::{Actor, ActorContext, StreamHandler};
use actix_web_actors::ws::{self, WebsocketContext};

/// Eine WebSocket-Verbindung zu einem Browser.
/// Pro verbundemen Browser gibt es eine Session.
pub struct ChatSession;

impl Actor for ChatSession {
    type Context = WebsocketContext<Self>;
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ChatSession {
    fn handle(
        &mut self,
        item: Result<ws::Message, ws::ProtocolError>,
        context: &mut Self::Context,
    ) {
        match item {
            Ok(ws::Message::Text(text)) => context.text(text),
            Ok(ws::Message::Close(reason)) => {
                context.close(reason);
                context.stop()
            }
            Err(_) => context.stop(),
            _ => {}
        }
    }
}
