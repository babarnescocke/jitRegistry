#![allow(non_snake_case)]
use crate::clilib::cliargs::Args;
use actix_web::{get, web, App, HttpRequest, HttpServer, Responder};
use std::io;
pub mod buildah;
pub mod clilib;
//use oci_spec::image::ImageManifest;

#[actix_web::main]
async fn main() -> io::Result<()> {
    let arg = Args::args_or_exit();
    HttpServer::new(|| {
        App::new()
            .route("/v2/", web::get().to(|| async { "" }))
            .service(name)
            .service(blobs)
    })
    .bind((arg.bind_addr, arg.bind_port))?
    .run()
    .await
}
/// Service takes a pull manifest request, per https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pulling-manifests
#[get("/v2/{name}/manifest/latest")]
async fn name(name: web::Path<String>) -> impl Responder {
    todo!();
}

fn get_content_type<'a>(req: &'a HttpRequest) -> Option<&'a str> {
    req.headers().get("content-type")?.to_str().ok()
}
/// Service serves blobs: https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pulling-blobs
#[get("/v2/{name}/blobs/{reference}")]
async fn blobs(blob: web::Path<String>) -> impl Responder {
    todo!();
}
