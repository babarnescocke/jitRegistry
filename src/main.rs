#![allow(non_snake_case)]
use crate::buildah::b::{
    buildah_dockerconatinerfile_build, buildah_unshare_build, hash_to_manifest,
    pathbuf_to_actionable_buildah_path,
};
use crate::clilib::cliargs::{Args, WA};
use actix_web::{get, http::header, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use std::io;
pub mod buildah;
pub mod clilib;

#[actix_web::main]
async fn main() -> std::result::Result<(), io::Error> {
    let arg = Args::args_or_exit();
    let wa: web::Data<WA> = arg.args_to_data_wa();
    HttpServer::new(move || {
        App::new()
            .app_data(wa.to_owned())
            .service(buildah_build)
            .service(light_is_on)
    })
    .bind((arg.bind_addr, arg.bind_port))?
    .run()
    .await
}

/// This gets pinged by podman when pulling an image, to verify this is compliant with OCI spec v2.
#[get("/v2/")]
async fn light_is_on() -> impl Responder {
    HttpResponse::Ok().body("")
}

/// Service takes a pull manifest request, per https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pulling-manifests
#[get("/v2/{name}/manifests/latest")]
async fn buildah_build(name: web::Path<String>, data: web::Data<WA>) -> impl Responder {
    let (buildah_script_opt, somefile_opt) =
        pathbuf_to_actionable_buildah_path(&data.con_dir_path, &name).expect("Cannot Access Path");
    let mut hash = String::from("");

    match buildah_script_opt {
        Some(x) => hash = buildah_unshare_build(&x).expect("buildah error"),
        None => {}
    }
    match somefile_opt {
        Some(x) => hash = buildah_dockerconatinerfile_build(&x).expect("buildah error"),
        None => {}
    }
    if !hash.eq("") {
        let mut clone = (&data.buildah_dir).clone();
        let ret = hash_to_manifest(&hash, &mut clone).expect("something good");
        let clone = ret.clone();
        dbg!(clone);
        // need to use this https://docs.rs/oci-spec/0.5.5/src/oci_spec/image/mod.rs.html#60 to get correct json info.
        HttpResponse::Ok().body(serde_json::to_string(&ret).unwrap())
    } else {
        HttpResponse::UnsupportedMediaType().body("something_not good")
    }
}

/// reads content-type header
fn get_content_type<'a>(req: &'a HttpRequest) -> Option<&'a str> {
    req.headers().get("content-type")?.to_str().ok()
}
// Service serves blobs: https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pulling-blobs
//#[get("/v2/{name}/blobs/{reference}")]
//async fn blobs(blob: web::Path<String>, wa: web::Data<WA>) -> impl Responder {
//    todo!();
//}
