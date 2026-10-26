//! Eine `ChatSession` pro WebSocket-Verbindung.
//! Übersetzt zwischen dem JSON des Browsers
//! und den Actor-Nachrichten des `ChatServer`.

use std::time::{Duration, Instant};

use actix::{Actor, ActorContext, Addr, AsyncContext, StreamHandler};
use actix::{ActorFutureExt, Handler, WrapFuture};
use actix_web_actors::ws::{self, WebsocketContext};

use crate::chat_room::ChatMessage;
use crate::protocol::{ErrorCode, IncomingMessage, OutgoingMessage};
use crate::server::{ChatServer, ClientMessage, Connect, Disconnect, GetHistory, GetRooms, ServerMessage};

/// Eine WebSocket-Verbindung zu einem Browser.
/// Hat zwei Zustände: verbunden, aber noch nicht angemeldet (`client_id` ist `None`),
/// und angemeldet. Erst nach der Anmeldung kennt der `ChatServer` die Session.
pub struct ChatSession {
    /// Letztes Lebenszeichen (Ping oder Pong) des Browsers, für den Heartbeat.
    last_heard_from_client: Instant,
    server_address: Addr<ChatServer>,
    /// Wird vom `ChatServer` vergeben, nie vom Browser übernommen.
    client_id: Option<u64>,
}

/// Wie oft die Session einen Ping schickt.
const PING_TIME: std::time::Duration = Duration::from_secs(5);
/// Ohne Lebenszeichen in dieser Zeit gilt die Verbindung als tot.
const WAIT_TIME: std::time::Duration = Duration::from_secs(10);

impl Actor for ChatSession {
    type Context = WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        println!("Session gestartet.");
        // Heartbeat: ein zugeklappter Laptop schickt kein Close-Frame.
        // Ohne diese Prüfung bliebe die Session für immer bestehen.
        ctx.run_interval(PING_TIME, |session, ctx| {
            if session.last_heard_from_client.elapsed() > WAIT_TIME {
                println!("Heartbeat-Timeout, Verbindung wird getrennt.");
                ctx.stop();
                return;
            }
            ctx.ping(b"");
        });
    }

    /// Läuft bei jedem Ende der Session: Close, Heartbeat-Timeout oder Fehler.
    /// Deshalb ist das der einzige verlässliche Ort für den `Disconnect`.
    fn stopped(&mut self, _ctx: &mut Self::Context) {
        println!("Session wurde beendet.");
        // Ohne Anmeldung war die Session beim ChatServer nie registriert.
        let Some(id) = self.client_id else {
            return;
        };

        self.server_address.do_send(Disconnect { client_id: id });
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

    /// Schickt eine Nachricht als JSON an den Browser.
    /// Fehler werden nur geloggt, damit der Server nicht abstürzt.
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

    /// Kaputtes JSON und unbekannte Nachrichtentypen ergeben beide `invalid_json`.
    /// Serde unterscheidet die beiden Fälle nicht ohne Weiteres.
    pub fn handle_incoming_text(&mut self, text: &str, ctx: &mut WebsocketContext<Self>) {
        println!("Nachricht vom Client empfangen: {text}");
        let data: Result<IncomingMessage, serde_json::Error> = serde_json::from_str(text);

        match data {
            Ok(IncomingMessage::Login { name }) => self.handle_login(name, ctx),
            Ok(IncomingMessage::Chat { text, room }) => self.handle_chat(text, room, ctx),
            Ok(IncomingMessage::History { room }) => self.handle_history(room, ctx),
            //* 
            Err(error) => {
                println!("JSON-Parsing fehlgeschlagen: {error}");
                self.sending_outgoing_to_browser(
                ctx,
                OutgoingMessage::Error {
                    code: ErrorCode::InvalidJson,
                    message: "Invalid JSON was sent in.".to_string(),
                },
            ) }
        }
    }

    fn load_chatrooms(&mut self, ctx: &mut WebsocketContext<Self>) {
        let server_address = self.server_address.clone();

        let future = server_address.send(GetRooms).into_actor(self)
            .map(|result, session, ctx| {
                match result {
                    Ok(rooms) => {
                        session.sending_outgoing_to_browser(
                            ctx,
                            OutgoingMessage::RoomList {rooms },
                        );}
                    Err(error) => {
                        eprintln!("Fehler beim Laden der Raumliste: {}",error
                        );}
                }
            });

        ctx.wait(future);
    }

    pub fn handle_history(&mut self, room: String, ctx: &mut WebsocketContext<Self>){
        println!("handle-history aufgerufen");
        let Some(id) = self.client_id else {
            self.sending_outgoing_to_browser(ctx, OutgoingMessage::Error {
                    code: ErrorCode::AlreadyLoggedIn,
                    message: "Client is already logged in.".to_string(),
                });
            return;
        };

        let requested_room = room.clone();

        let future = self.server_address.send(GetHistory {
            room, client_id: id
        }).into_actor(self)
        .map(move |result, session, ctx|{
            match result {
                Ok(Some(messages)) => {
                    let messages: Vec<ChatMessage> = messages.into_iter().map(|message| ChatMessage {
                        sender_id: message.sender_id,
                        sender_name: message.sender_name,
                        text: message.text,
                        timestamp: message.timestamp,
                    }).collect();

                    session.sending_outgoing_to_browser(ctx, OutgoingMessage::History { room: requested_room, messages });
                }
                Ok(None) => {
                    eprintln!("Fehler beim Laden des Verlaufs: Raum nicht gefunden oder Client ist kein Mitglied");
                }
                Err(error) => {
                    eprintln!("Fehler beim Laden des Verlaufs: {}", error);
                    ctx.stop();
                }
            }
        });
        ctx.wait(future);

    }

    /// Prüft die Anmeldung, registriert die Session beim `ChatServer`
    /// und schickt dem Browser seine ID.
    pub fn handle_login(&mut self, name: String, ctx: &mut WebsocketContext<Self>) {
        println!("handle-login wurde aufgerufen");
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

        // Die Antwort auf `Connect` kommt erst später. Da Actor-Methoden nicht `async` sind,
        // läuft der Rest in einer Closure, die Actix mit der Session (`session`) aufruft,
        // denn `self` gibt es dann nicht mehr.
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
                    println!("Anmeldung erfolgreich: {}!", name);
                    session.sending_outgoing_to_browser(ctx, OutgoingMessage::Welcome { id, name });

                    //Chatrooms beim Chatserver abfragen
                    session.load_chatrooms(ctx);
                }
                Err(_) => {
                    println!("ChatServer nicht erreichbar, Session wird beendet.");
                    ctx.stop();
                }
            });

        // `wait` statt `spawn`: Bis die ID da ist, verarbeitet die Session nichts anderes.
        // Sonst könnte eine direkt folgende Chatnachricht „nicht angemeldet“ ergeben.
        ctx.wait(actor_future);
    }

    /// Leitet eine Chatnachricht an den `ChatServer` weiter.
    /// Leere oder zu lange Texte verwirft der `ChatServer` selbst.
    pub fn handle_chat(&mut self, text: String, room: String,ctx: &mut WebsocketContext<Self>) {
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

        // Im MVP gibt es nur den Raum "welcome".
        self.server_address.do_send(ClientMessage {
            id,
            msg: text,
            room: room,
        });
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ChatSession {
    fn handle(&mut self, item: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match item {
            Ok(ws::Message::Text(text)) => self.handle_incoming_text(&text, ctx),
            Ok(ws::Message::Close(reason)) => {
                println!("Browser sendet Close-Frame: {:?}", reason);
                ctx.close(reason);
                ctx.stop()
            }
            Err(error) => {
                eprintln!("Websocket ProkotolError: {:?}", error);
                ctx.stop();
            }
            // Ping und Pong zählen als Lebenszeichen für den Heartbeat.
            Ok(ws::Message::Ping(bytes)) => {
                self.last_heard_from_client = Instant::now();
                ctx.pong(&bytes)
            }
            Ok(ws::Message::Pong(_)) => self.last_heard_from_client = Instant::now(),
            // Das Protokoll erlaubt nur JSON als Text.
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

/// Nachrichten vom `ChatServer` werden ins Protokollformat übersetzt.
/// So bleibt der `ChatServer` unabhängig vom JSON-Format.
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
