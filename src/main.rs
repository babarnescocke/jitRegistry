#![allow(non_snake_case)]

use actix_web::{get, web, App, HttpServer, Responder};
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use structopt::StructOpt;
pub mod clilib;

#[actix_web::main]
async fn main() -> io::Result<()> {
    let arg = clilib::cliargs::cli_return_or_error_exit();
    HttpServer::new(|| {
        App::new()
            .route("/containers", web::get().to(|| async { "Hello World!" }))
            .service(greet)
    })
    .bind(("127.0.0.1", 7999))?
    .run()
    .await
}

#[get("/containers/{name}")]
async fn greet(name: web::Path<String>) -> impl Responder {
    format!("Hello {name}!")
}
