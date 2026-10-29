//! One actor per WebSocket connection. Translates JSON to server messages.
use crate::protocol::{ErrorCode, IncomingMessage, OutgoingMessage};
use crate::server::{
    ChatServer, ClientMessage, Connect, Disconnect, GetHistory, GetRoomState, ManageRoom,
    RoomAction, ServerMessage,
};
use actix::{Actor, ActorContext, Addr, AsyncContext, StreamHandler};
use actix::{ActorFutureExt, Handler, WrapFuture};
use actix_web_actors::ws::{self, WebsocketContext};
use std::time::{Duration, Instant};

pub struct ChatSession {
    last_heard_from_client: Instant,
    server_address: Addr<ChatServer>,
    client_id: Option<u64>,
}

const PING_TIME: Duration = Duration::from_secs(5);
const WAIT_TIME: Duration = Duration::from_secs(10);

impl Actor for ChatSession {
    type Context = WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(PING_TIME, |session, ctx| {
            if session.last_heard_from_client.elapsed() > WAIT_TIME {
                ctx.stop();
                return;
            }
            ctx.ping(b"");
        });
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        if let Some(id) = self.client_id {
            self.server_address.do_send(Disconnect { client_id: id });
        }
    }
}

impl ChatSession {
    pub fn new(server_address: Addr<ChatServer>) -> Self {
        Self {
            last_heard_from_client: Instant::now(),
            server_address,
            client_id: None,
        }
    }

    pub fn sending_outgoing_to_browser(
        &mut self,
        ctx: &mut WebsocketContext<Self>,
        message: OutgoingMessage,
    ) {
        match serde_json::to_string(&message) {
            Ok(json) => ctx.text(json),
            Err(error) => eprintln!("Could not serialize response: {error}"),
        }
    }

    fn send_error(&mut self, ctx: &mut WebsocketContext<Self>, code: ErrorCode, message: &str) {
        self.sending_outgoing_to_browser(
            ctx,
            OutgoingMessage::Error {
                code,
                message: message.to_string(),
            },
        );
    }

    pub fn handle_incoming_text(&mut self, text: &str, ctx: &mut WebsocketContext<Self>) {
        match serde_json::from_str::<IncomingMessage>(text) {
            Ok(IncomingMessage::Login { name }) => self.handle_login(name, ctx),
            Ok(IncomingMessage::Chat { text, room }) => self.handle_chat(text, room, ctx),
            Ok(IncomingMessage::History { room }) => self.handle_history(room, ctx),
            Ok(IncomingMessage::CreateRoom { room }) => {
                self.handle_room_action(room, RoomAction::Create, ctx);
            }
            Ok(IncomingMessage::JoinRoom { room }) => {
                self.handle_room_action(room, RoomAction::Join, ctx);
            }
            Ok(IncomingMessage::LeaveRoom { room }) => {
                self.handle_room_action(room, RoomAction::Leave, ctx);
            }
            Ok(IncomingMessage::ListRooms) => self.load_chatrooms(ctx),
            Err(error) => {
                eprintln!("Invalid incoming JSON: {error}");
                self.send_error(ctx, ErrorCode::InvalidJson, "Invalid JSON or message type.");
            }
        }
    }

    fn load_chatrooms(&mut self, ctx: &mut WebsocketContext<Self>) {
        let Some(id) = self.client_id else {
            self.send_error(ctx, ErrorCode::NotLoggedIn, "Please log in first.");
            return;
        };
        let future = self
            .server_address
            .send(GetRoomState { client_id: id })
            .into_actor(self)
            .map(|result, session, ctx| match result {
                Ok(state) => session.sending_outgoing_to_browser(
                    ctx,
                    OutgoingMessage::RoomList {
                        rooms: state.rooms,
                        joined_rooms: state.joined_rooms,
                    },
                ),
                Err(error) => {
                    eprintln!("Could not load rooms: {error}");
                    ctx.stop();
                }
            });
        ctx.wait(future);
    }

    fn handle_room_action(
        &mut self,
        room: String,
        action: RoomAction,
        ctx: &mut WebsocketContext<Self>,
    ) {
        let Some(id) = self.client_id else {
            self.send_error(ctx, ErrorCode::NotLoggedIn, "Please log in first.");
            return;
        };
        let leaving = matches!(&action, RoomAction::Leave);
        let future = self
            .server_address
            .send(ManageRoom {
                client_id: id,
                room,
                action,
            })
            .into_actor(self)
            .map(move |result, session, ctx| match result {
                Ok(Ok(room)) => {
                    let response = if leaving {
                        OutgoingMessage::RoomLeft { room }
                    } else {
                        OutgoingMessage::RoomJoined { room }
                    };
                    session.sending_outgoing_to_browser(ctx, response);
                    session.load_chatrooms(ctx);
                }
                Ok(Err(message)) => {
                    session.send_error(ctx, ErrorCode::RoomOperationFailed, &message);
                }
                Err(error) => {
                    eprintln!("Could not manage room: {error}");
                    ctx.stop();
                }
            });
        ctx.wait(future);
    }

    pub fn handle_history(&mut self, room: String, ctx: &mut WebsocketContext<Self>) {
        let Some(id) = self.client_id else {
            self.send_error(ctx, ErrorCode::NotLoggedIn, "Please log in first.");
            return;
        };
        let requested_room = room.clone();
        let future = self
            .server_address
            .send(GetHistory {
                room,
                client_id: id,
            })
            .into_actor(self)
            .map(move |result, session, ctx| match result {
                Ok(Some(messages)) => session.sending_outgoing_to_browser(
                    ctx,
                    OutgoingMessage::History {
                        room: requested_room,
                        messages,
                    },
                ),
                Ok(None) => session.send_error(
                    ctx,
                    ErrorCode::RoomOperationFailed,
                    "Room does not exist or you are not a member.",
                ),
                Err(error) => {
                    eprintln!("Could not load history: {error}");
                    ctx.stop();
                }
            });
        ctx.wait(future);
    }

    pub fn handle_login(&mut self, name: String, ctx: &mut WebsocketContext<Self>) {
        if self.client_id.is_some() {
            self.send_error(
                ctx,
                ErrorCode::AlreadyLoggedIn,
                "Client is already logged in.",
            );
            return;
        }
        let name = name.trim().to_string();
        if name.is_empty() || name.chars().count() > 20 {
            self.send_error(
                ctx,
                ErrorCode::InvalidName,
                "Name must contain 1-20 characters.",
            );
            return;
        }
        let future = self
            .server_address
            .send(Connect {
                client_addr: ctx.address().recipient(),
                name: name.clone(),
            })
            .into_actor(self)
            .map(move |result, session, ctx| match result {
                Ok(id) => {
                    session.client_id = Some(id);
                    session.sending_outgoing_to_browser(ctx, OutgoingMessage::Welcome { id, name });
                    session.load_chatrooms(ctx);
                }
                Err(error) => {
                    eprintln!("Could not log in: {error}");
                    ctx.stop();
                }
            });
        ctx.wait(future);
    }

    pub fn handle_chat(&mut self, text: String, room: String, ctx: &mut WebsocketContext<Self>) {
        let Some(id) = self.client_id else {
            self.send_error(ctx, ErrorCode::NotLoggedIn, "Please log in first.");
            return;
        };
        self.server_address.do_send(ClientMessage {
            id,
            msg: text,
            room,
        });
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ChatSession {
    fn handle(&mut self, item: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match item {
            Ok(ws::Message::Text(text)) => self.handle_incoming_text(&text, ctx),
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            Ok(ws::Message::Ping(bytes)) => {
                self.last_heard_from_client = Instant::now();
                ctx.pong(&bytes);
            }
            Ok(ws::Message::Pong(_)) => {
                self.last_heard_from_client = Instant::now();
            }
            Ok(ws::Message::Binary(_)) => {
                self.send_error(
                    ctx,
                    ErrorCode::InvalidJson,
                    "Send JSON as text, not binary.",
                );
            }
            Err(error) => {
                eprintln!("WebSocket error: {error}");
                ctx.stop();
            }
            _ => {}
        }
    }
}

impl Handler<ServerMessage> for ChatSession {
    type Result = ();
    fn handle(&mut self, msg: ServerMessage, ctx: &mut WebsocketContext<Self>) {
        self.sending_outgoing_to_browser(
            ctx,
            OutgoingMessage::Chat {
                sender_id: msg.sender_id,
                sender_name: msg.sender_name,
                text: msg.text,
                room: msg.room,
                timestamp: msg.timestamp,
            },
        );
    }
}
