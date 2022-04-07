#![allow(non_snake_case)]
use crate::clilib::cliargs::Args;
use actix_web::{get, web, App, HttpServer, Responder};
use std::io;
pub mod buildah;
pub mod clilib;
use oci_spec::image::ImageManifest;

#[actix_web::main]
async fn main() -> io::Result<()> {
    let arg = Args::args_or_exit();
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
