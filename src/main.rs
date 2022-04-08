#![allow(non_snake_case)]
use crate::buildah::b::{buildah_dockerconatinerfile_build, buildah_unshare_build};
use crate::clilib::cliargs::{Args, WA};
use actix_web::{get, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use std::io;
use std::path::PathBuf;
use walkdir::WalkDir;
pub mod buildah;
pub mod clilib;

#[actix_web::main]
async fn main() -> io::Result<()> {
    let arg = Args::args_or_exit();
    let wa: web::Data<WA> = arg.args_to_data_wa();
    HttpServer::new(move || App::new().app_data(wa.to_owned()).service(buildah_build))
        .bind((arg.bind_addr, arg.bind_port))?
        .run()
        .await
}
// Service takes a pull manifest request, per https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pulling-manifests
#[get("/v2/{name}/manifest/latest")]
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
        HttpResponse::Ok().body(hash)
    } else {
        HttpResponse::Ok().body("error, you goofed")
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

///Takes a pathbuf and then it returns either a the route to a valid shell script or a dir to run docker OR an error.
//pub fn pathbuf_to_actionable_buildah(
//    p: PathBuf,
//) -> Result<(PathBuf, PathBuf), Box<dyn std::error::Error>> {
//}

/// At time of this writing, pathbuf doesn't have a try_exists in stable -so we are just going to use exists for now, it obviously flattens errors
fn PathBuf_has_sub_dir(pb: &mut PathBuf, sd: &str) -> bool {
    pb.push(sd);
    pb.is_dir()
}

/// takes a PathBuf and says we either have an executable buildah script script, dockerfile/containerfile or an error
fn pathbuf_to_actionable_buildah_path(
    pb: &PathBuf,
    sd: &str,
) -> Result<(Option<PathBuf>, Option<PathBuf>), Box<dyn std::error::Error>> {
    if PathBuf_has_sub_dir(&mut (pb.clone()), sd.clone()) {
        let mut path = pb.clone();
        path.push(sd);
        for f in WalkDir::new(path.clone()).max_depth(1) {
            let F = f?;
            let E = F.clone();
            let metadata = (F.clone()).metadata()?;
            if metadata.is_file() {
                let testr: &str = E.file_name().to_str().unwrap();
                let testrclone0 = testr.clone();
                let testrclone1 = testr.clone();
                if testrclone0.starts_with("Dockerfile") || testrclone1.starts_with("Containerfile")
                {
                    return Ok((None, Some(path)));
                } else if testr.ends_with("sh") {
                    return Ok((Some(F.into_path()), None));
                }
            }
        }
        /*        for dir_item in read_dir(path)? {
            let dir_item = dir_item?
            if dir_item
            if dir_item
                .clone()
                .file_name
                .to_str()
                .unwrap()
                .starts_with("Containerfile")
                || dir_item
                    .file_name
                    .to_str()
                    .unwrap()
                    .starts_with("Dockerfile")
            {
                return Ok((None, Some(dir_item.to_path())));
            }
        }*/
    }
    Err(format!("Cannot find a subdirectory for path: {:?}", pb))?
}
