use actix::prelude::*;
use actix_files::Files;
use actix_web::{App, HttpServer, web};

mod chat_room;
mod server;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let chat_server = server::ChatServer::new().start();

    let app_state = web::Data::new(chat_server);

    println!("Server running at http://127.0.0.1:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(Files::new("/", "./static").index_file("index.html"))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
