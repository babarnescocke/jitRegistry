#![allow(non_snake_case)]

use actix_web::{get, web, App, HttpServer, Responder};
use std::io;
pub mod clilib;

#[actix_web::main]
async fn main() -> io::Result<()> {
    let arg = clilib::cliargs::cli_return_or_error_exit();
    HttpServer::new(|| {
        App::new()
            .route("/containers", web::get().to(|| async { "Hello World!" }))
            .service(greet)
    })
    .bind((arg.bind_addr, arg.bind_port))?
    .run()
    .await
}

#[get("/containers/{name}")]
async fn greet(name: web::Path<String>) -> impl Responder {
    format!("Hello {name}!")
}
