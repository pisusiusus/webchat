use std::time::{Duration, Instant};

use actix::{Actor, ActorContext, Addr, AsyncContext, StreamHandler};
use actix::{ActorFutureExt, Handler, WrapFuture};
use actix_web_actors::ws::{self, WebsocketContext};

use crate::protocol::{ErrorCode, IncomingMessage, OutgoingMessage};
use crate::server::{ChatServer, ClientMessage, Connect, Disconnect, ServerMessage};

/// Eine WebSocket-Verbindung zu einem Browser.
/// Pro verbundemen Browser gibt es eine Session.
pub struct ChatSession {
    last_heard_from_client: Instant,
    server_address: Addr<ChatServer>,
    client_id: Option<u64>,
}

const PING_TIME: std::time::Duration = Duration::from_secs(5);
const WAIT_TIME: std::time::Duration = Duration::from_secs(10);

impl Actor for ChatSession {
    type Context = WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        println!("Session gestartet.");
        ctx.run_interval(PING_TIME, |session, ctx| {
            if session.last_heard_from_client.elapsed() > WAIT_TIME {
                println!("Heartbeat-Timeout, Verbindung wird getrennt.");
                ctx.stop();
                return;
            }
            ctx.ping(b"");
        });
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        
        let Some(id) = self.client_id else {
            return;
        };

        self.server_address.do_send(Disconnect {
            client_id: id
        });

    }
}

impl ChatSession {
    pub fn new(server_address: Addr<ChatServer>) -> ChatSession {
        ChatSession {
            last_heard_from_client: Instant::now(),
            server_address,
            client_id: None,
        }
    }

    pub fn sending_outgoing_to_browser(
        &mut self,
        ctx: &mut WebsocketContext<Self>,
        outgoing_message: OutgoingMessage,
    ) {
        let json_data = serde_json::to_string(&outgoing_message);

        match json_data {
            Ok(json) => ctx.text(json),
            Err(error) => println!(
                "Nachricht konnte nicht in JSON umgewandelt werden: {}",
                error
            ),
        }
    }

    pub fn handle_incoming_text(&mut self, text: &str, ctx: &mut WebsocketContext<Self>) {
        let data: Result<IncomingMessage, serde_json::Error> = serde_json::from_str(text);

        match data {
            Ok(IncomingMessage::Login { name }) => self.handle_login(name, ctx),
            Ok(IncomingMessage::Chat { text }) => self.handle_chat(text, ctx),
            Err(_) => self.sending_outgoing_to_browser(
                ctx,
                OutgoingMessage::Error {
                    code: ErrorCode::InvalidJson,
                    message: "Invalid JSON was sent in.".to_string(),
                },
            ),
        }
    }

    pub fn handle_login(&mut self, name: String, ctx: &mut WebsocketContext<Self>) {
        if self.client_id.is_some() {
            self.sending_outgoing_to_browser(
                ctx,
                OutgoingMessage::Error {
                    code: ErrorCode::AlreadyLoggedIn,
                    message: "Client is already logged in.".to_string(),
                },
            );
            return;
        }

        if name.trim().is_empty() {
            self.sending_outgoing_to_browser(
                ctx,
                OutgoingMessage::Error {
                    code: ErrorCode::InvalidName,
                    message: "The client name was invalid".to_string(),
                },
            );
            return;
        }

        let client_addr = ctx.address().recipient();
        let client_name = name.clone();

        let actor_future = self
            .server_address
            .send(Connect {
                client_addr,
                name: client_name,
            })
            .into_actor(self)
            .map(move |result, session, ctx| match result {
                Ok(id) => {
                    session.client_id = Some(id);
                    println!("The Login was valid. Welcome, {}!", name);
                    session.sending_outgoing_to_browser(ctx, OutgoingMessage::Welcome { id, name });
                }
                Err(_) => {
                    println!("ChatServer is not reachable.");
                    ctx.stop();
                }
            });
        ctx.wait(actor_future);
    }

    pub fn handle_chat(&mut self, text: String, ctx: &mut WebsocketContext<Self>) {
        let Some(id) = self.client_id else {
            self.sending_outgoing_to_browser(
                ctx,
                OutgoingMessage::Error {
                    code: ErrorCode::NotLoggedIn,
                    message: "Client is not logged in.".to_string(),
                },
            );
            return;
        };

        self.server_address.do_send(ClientMessage {
            id,
            msg: text,
            room: "welcome".to_string(),
        });
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ChatSession {
    fn handle(&mut self, item: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match item {
            Ok(ws::Message::Text(text)) => self.handle_incoming_text(&text, ctx),
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop()
            }
            Err(_) => ctx.stop(),
            Ok(ws::Message::Ping(bytes)) => {
                self.last_heard_from_client = Instant::now();
                ctx.pong(&bytes)
            }
            Ok(ws::Message::Pong(_)) => self.last_heard_from_client = Instant::now(),
            Ok(ws::Message::Binary(_)) => self.sending_outgoing_to_browser(
                ctx,
                OutgoingMessage::Error {
                    code: ErrorCode::InvalidJson,
                    message: "InvalidJson was sent in.".to_string(),
                },
            ),
            _ => {}
        }
    }
}

impl Handler<ServerMessage> for ChatSession {
    type Result = ();

    fn handle(&mut self, msg: ServerMessage, ctx: &mut WebsocketContext<Self>) {
        let outgoing_message = OutgoingMessage::Chat {
            sender_id: msg.sender_id,
            sender_name: msg.sender_name,
            text: msg.text,
            room: msg.room,
            timestamp: msg.timestamp,
        };

        self.sending_outgoing_to_browser(ctx, outgoing_message);
    }
}
