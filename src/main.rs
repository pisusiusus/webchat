use actix_files::Files;
use actix_web::{App, Error, HttpRequest, HttpResponse, HttpServer, get, web};
use futures_util::StreamExt;

#[get("/ws")]
async fn ws_endpoint(
    req: HttpRequest,
    body: web::Payload
) -> Result<HttpResponse, Error> {
    let (response, mut session, mut stream) = actix_ws::handle(&req, body)?;

    actix_web::rt::spawn(async move {
        while let Some(Ok(msg)) = stream.next().await {
            if let actix_ws::Message::Text(text) = msg {
                if session.text(text).await.is_err() {
                    break;
                }
            }
        }
    });

    Ok(response)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Server läuft auf http://127.0.0.1:8080");
    HttpServer::new(|| {
        App::new()
            .service(ws_endpoint)
            .service(Files::new("/", "./static").index_file("index.html"))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}