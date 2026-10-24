use std::time::{Duration, Instant};

use actix::{Actor, ActorContext, AsyncContext, StreamHandler};
use actix_web_actors::ws::{self, WebsocketContext};

/// Eine WebSocket-Verbindung zu einem Browser.
/// Pro verbundemen Browser gibt es eine Session.
pub struct ChatSession {
    last_heard_from_client: Instant,
}

const PING_TIME: std::time::Duration = Duration::from_secs(5);
const WAIT_TIME: std::time::Duration = Duration::from_secs(10);

impl Actor for ChatSession {
    type Context = WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        println!("Session gestartet.");
        ctx.run_interval(PING_TIME, |session, ctx| {
            if session.last_heard_from_client.elapsed() > WAIT_TIME {
                ctx.ping(b"");
                println!("Heartbeat-Timeout, Verbindung wird getrennt.");
                ctx.stop();
                return
            }
        });
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        println!("Session beendet.");
    }
}

impl ChatSession {
    pub fn new() -> ChatSession {
        ChatSession {
            last_heard_from_client: Instant::now()
        }
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
            Ok(ws::Message::Ping(bytes)) => {self.last_heard_from_client = Instant::now();
                                                    ctx.pong(&bytes)},
            Ok(ws::Message::Pong(_)) => self.last_heard_from_client = Instant::now(), // TODO
            Ok(ws::Message::Binary(_)) => {}, // TODO
            _ => {}
        }
    }
}
