use actix::prelude::*;
use actix_files::Files;
use actix_web::{App, Error, HttpRequest, HttpResponse, HttpServer, web};
use actix_web_actors::ws;

mod chat_room;
mod server;
mod session;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let chat_server = server::ChatServer::new().start();

    let app_state = web::Data::new(chat_server);

    println!("Server running at http://127.0.0.1:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/ws", web::get().to(ws_route))
            .service(Files::new("/", "./static").index_file("index.html"))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

async fn ws_route(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    ws::start(session::ChatSession, &req, stream)
}
