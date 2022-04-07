#![allow(non_snake_case)]

use actix_web::{get, web, App, HttpServer, Responder};
use std::io;
pub mod buildah;
pub mod clilib;
use oci_spec::image::ImageManifest;

#[actix_web::main]
async fn main() -> io::Result<()> {
    //let image_manifest = ImageManifest::from_file("/.local/share/containers/storage/overlay-images/8b89c0950fee98cf1edc8ffa8400f756234809a70f7c2a927eab9891eb18d6bb/manifest").unwrap();
    match buildah::b::buildah_graph_root() {
        Ok(x) => println!("{:?}", x),
        Err(e) => eprintln!("Error {}", e),
    }
    Ok(())
    //HttpServer::new(|| {
    //    App::new()
    //        .route("/containers", web::get().to(|| async { "Hello World!" }))
    //        .service(greet)
    //})
    //bind((arg.bind_addr, arg.bind_port))?
    //.run()
    //.await
}

#[get("/containers/{name}")]
async fn greet(name: web::Path<String>) -> impl Responder {
    format!("Hello {name}!")
}
