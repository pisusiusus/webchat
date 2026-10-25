use actix::prelude::*;
use actix_files::Files;
use actix_web::{App, Error, HttpRequest, HttpResponse, HttpServer, web};
use actix_web_actors::ws;

use crate::{server::ChatServer, session::ChatSession};

mod chat_room;
mod protocol;
mod server;
mod session;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Der ChatServer wird genau einmal gestartet, außerhalb von `HtppServer::new`.
    // Die Closure dort läuft einmal pro Worker-Thread. Stünde der Start darin,
    // hätte jeder Worker seinen eigenen ChatServer, und Nutzer auf
    // verschiedenen Workern würden sich nicht sehen.
    let chat_server = server::ChatServer::new().start();

    let app_state = web::Data::new(chat_server);

    println!("Server läuft auf http://127.0.0.1:8080");

    HttpServer::new(move || {
        App::new()
            // Jeder Worker bekommt eine Kopie der Adresse, nicht des ChatServers.
            .app_data(app_state.clone())
            // Muss vor `Files` stehen: `Files` auf "/" fängt sonst alle Anfragen ab.
            .route("/ws", web::get().to(ws_route))
            .service(Files::new("/", "./static").index_file("index.html"))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

/// Nimmt eine WebSocket-Verbindung an und übergibt sie an eine neue `ChatSession`.
/// Pro Verbindung entsteht eine eigene Session.
async fn ws_route(
    req: HttpRequest,
    stream: web::Payload,
    shared_address: web::Data<Addr<ChatServer>>,
) -> Result<HttpResponse, Error> {
    ws::start(
        ChatSession::new(shared_address.get_ref().clone()),
        &req,
        stream,
    )
}
