#![allow(non_snake_case)]
use crate::buildah::b::{
    blob_path, buildah_dockerconatinerfile_build, buildah_push_to_oci_layout,
    buildah_unshare_build, pathbuf_to_actionable_buildah_path, read_oci_index_manifest_digest,
};
use crate::clilib::cliargs::{Args, WA};
use crate::oci::oci_helpers::{digest_hex, is_digest_reference, oci_error, sha256_digest};
use actix_web::{web, App, HttpResponse, HttpServer};
use std::io;
pub mod buildah;
pub mod clilib;
pub mod oci;

#[actix_web::main]
async fn main() -> std::result::Result<(), io::Error> {
    let arg = Args::args_or_exit();
    let wa: web::Data<WA> = arg.args_to_data_wa();
    HttpServer::new(move || {
        App::new()
            .app_data(wa.to_owned())
            // Pull endpoints (required by OCI spec)
            .service(
                web::resource("/v2/")
                    .route(web::get().to(v2_base))
            )
            .service(
                web::resource("/v2/{name:.*}/manifests/{reference}")
                    .route(web::get().to(get_manifest))
                    .route(web::head().to(head_manifest))
                    .route(web::put().to(unsupported))
                    .route(web::delete().to(unsupported))
            )
            .service(
                web::resource("/v2/{name:.*}/blobs/uploads/{reference}")
                    .route(web::patch().to(unsupported))
                    .route(web::put().to(unsupported))
                    .route(web::delete().to(unsupported))
                    .route(web::get().to(unsupported))
            )
            .service(
                web::resource("/v2/{name:.*}/blobs/uploads/")
                    .route(web::post().to(unsupported))
            )
            .service(
                web::resource("/v2/{name:.*}/blobs/{digest}")
                    .route(web::get().to(get_blob))
                    .route(web::head().to(head_blob))
                    .route(web::delete().to(unsupported))
            )
    })
    .bind((arg.bind_addr, arg.bind_port))?
    .run()
    .await
}

/// GET /v2/ — OCI spec base endpoint (end-1).
async fn v2_base() -> HttpResponse {
    HttpResponse::Ok()
        .insert_header(("Docker-Distribution-API-Version", "registry/2.0"))
        .json(serde_json::json!({}))
}

/// Builds an image by name (tag reference), exports to OCI layout, returns manifest bytes + digest.
fn build_and_export(
    name: &str,
    data: &web::Data<WA>,
) -> Result<(Vec<u8>, String), HttpResponse> {
    let (buildah_script_opt, somefile_opt) =
        pathbuf_to_actionable_buildah_path(&data.con_dir_path, name)
            .map_err(|_| oci_error(actix_web::http::StatusCode::NOT_FOUND, "NAME_UNKNOWN", "repository name not known to registry"))?;

    let mut hash = String::new();
    if let Some(ref x) = buildah_script_opt {
        hash = buildah_unshare_build(x)
            .map_err(|e| oci_error(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR, "MANIFEST_UNKNOWN", &e.to_string()))?;
    }
    if let Some(ref x) = somefile_opt {
        hash = buildah_dockerconatinerfile_build(x)
            .map_err(|e| oci_error(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR, "MANIFEST_UNKNOWN", &e.to_string()))?;
    }

    if hash.is_empty() {
        return Err(oci_error(
            actix_web::http::StatusCode::NOT_FOUND,
            "MANIFEST_UNKNOWN",
            "no buildable content found",
        ));
    }

    // Export to OCI layout
    buildah_push_to_oci_layout(&hash, &data.oci_cache_dir, name)
        .map_err(|e| oci_error(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR, "MANIFEST_UNKNOWN", &e.to_string()))?;

    // Read manifest digest from index.json
    let manifest_digest = read_oci_index_manifest_digest(&data.oci_cache_dir, name)
        .map_err(|e| oci_error(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR, "MANIFEST_UNKNOWN", &e.to_string()))?;

    let hex = digest_hex(&manifest_digest)
        .ok_or_else(|| oci_error(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR, "MANIFEST_UNKNOWN", "invalid digest in index.json"))?;

    let manifest_path = blob_path(&data.oci_cache_dir, name, hex);
    let manifest_bytes = std::fs::read(&manifest_path)
        .map_err(|e| oci_error(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR, "MANIFEST_UNKNOWN", &e.to_string()))?;

    let computed_digest = sha256_digest(&manifest_bytes);
    Ok((manifest_bytes, computed_digest))
}

/// Resolves a manifest by reference (tag or digest). Returns (bytes, digest).
fn resolve_manifest(
    name: &str,
    reference: &str,
    data: &web::Data<WA>,
) -> Result<(Vec<u8>, String), HttpResponse> {
    if is_digest_reference(reference) {
        // Digest-based lookup: find in cache
        let hex = digest_hex(reference)
            .ok_or_else(|| oci_error(actix_web::http::StatusCode::BAD_REQUEST, "DIGEST_INVALID", "invalid digest format"))?;
        let path = blob_path(&data.oci_cache_dir, name, hex);
        let bytes = std::fs::read(&path)
            .map_err(|_| oci_error(actix_web::http::StatusCode::NOT_FOUND, "MANIFEST_UNKNOWN", "manifest unknown to registry"))?;
        let computed = sha256_digest(&bytes);
        Ok((bytes, computed))
    } else {
        // Tag-based: build and export
        build_and_export(name, data)
    }
}

/// GET /v2/{name}/manifests/{reference} — Pull manifest (end-3).
async fn get_manifest(
    path: web::Path<(String, String)>,
    data: web::Data<WA>,
) -> HttpResponse {
    let (name, reference) = path.into_inner();
    match resolve_manifest(&name, &reference, &data) {
        Ok((bytes, digest)) => {
            let content_type = detect_manifest_content_type(&bytes);
            HttpResponse::Ok()
                .insert_header(("Content-Type", content_type))
                .insert_header(("Docker-Content-Digest", digest.as_str()))
                .insert_header(("Docker-Distribution-API-Version", "registry/2.0"))
                .body(bytes)
        }
        Err(resp) => resp,
    }
}

/// HEAD /v2/{name}/manifests/{reference} — Check manifest existence (end-3).
async fn head_manifest(
    path: web::Path<(String, String)>,
    data: web::Data<WA>,
) -> HttpResponse {
    let (name, reference) = path.into_inner();
    match resolve_manifest(&name, &reference, &data) {
        Ok((bytes, digest)) => {
            let content_type = detect_manifest_content_type(&bytes);
            HttpResponse::Ok()
                .insert_header(("Content-Type", content_type))
                .insert_header(("Docker-Content-Digest", digest.as_str()))
                .insert_header(("Docker-Distribution-API-Version", "registry/2.0"))
                .insert_header(("Content-Length", bytes.len().to_string()))
                .finish()
        }
        Err(resp) => resp,
    }
}

/// Reads the mediaType field from manifest JSON to determine Content-Type.
fn detect_manifest_content_type(bytes: &[u8]) -> &'static str {
    if let Ok(v) = serde_json::from_slice::<serde_json::Value>(bytes) {
        if let Some(mt) = v.get("mediaType").and_then(|m| m.as_str()) {
            if mt.contains("docker") {
                return "application/vnd.docker.distribution.manifest.v2+json";
            }
            if mt.contains("image.index") {
                return "application/vnd.oci.image.index.v1+json";
            }
        }
    }
    "application/vnd.oci.image.manifest.v1+json"
}

/// GET /v2/{name}/blobs/{digest} — Pull blob (end-2).
async fn get_blob(
    path: web::Path<(String, String)>,
    data: web::Data<WA>,
) -> HttpResponse {
    let (name, digest) = path.into_inner();
    let hex = match digest_hex(&digest) {
        Some(h) => h.to_string(),
        None => {
            return oci_error(
                actix_web::http::StatusCode::BAD_REQUEST,
                "DIGEST_INVALID",
                "invalid digest format",
            );
        }
    };

    let path = blob_path(&data.oci_cache_dir, &name, &hex);
    match std::fs::read(&path) {
        Ok(bytes) => {
            HttpResponse::Ok()
                .insert_header(("Content-Type", "application/octet-stream"))
                .insert_header(("Docker-Content-Digest", digest.as_str()))
                .insert_header(("Content-Length", bytes.len().to_string()))
                .body(bytes)
        }
        Err(_) => oci_error(
            actix_web::http::StatusCode::NOT_FOUND,
            "BLOB_UNKNOWN",
            "blob unknown to registry",
        ),
    }
}

/// HEAD /v2/{name}/blobs/{digest} — Check blob existence (end-2).
async fn head_blob(
    path: web::Path<(String, String)>,
    data: web::Data<WA>,
) -> HttpResponse {
    let (name, digest) = path.into_inner();
    let hex = match digest_hex(&digest) {
        Some(h) => h.to_string(),
        None => {
            return oci_error(
                actix_web::http::StatusCode::BAD_REQUEST,
                "DIGEST_INVALID",
                "invalid digest format",
            );
        }
    };

    let path = blob_path(&data.oci_cache_dir, &name, &hex);
    match std::fs::metadata(&path) {
        Ok(meta) => {
            HttpResponse::Ok()
                .insert_header(("Content-Type", "application/octet-stream"))
                .insert_header(("Docker-Content-Digest", digest.as_str()))
                .insert_header(("Content-Length", meta.len().to_string()))
                .finish()
        }
        Err(_) => oci_error(
            actix_web::http::StatusCode::NOT_FOUND,
            "BLOB_UNKNOWN",
            "blob unknown to registry",
        ),
    }
}

/// Returns 405 Method Not Allowed with OCI UNSUPPORTED error for read-only operations.
async fn unsupported() -> HttpResponse {
    oci_error(
        actix_web::http::StatusCode::METHOD_NOT_ALLOWED,
        "UNSUPPORTED",
        "this registry is read-only",
    )
}
