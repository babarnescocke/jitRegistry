#![allow(non_snake_case)]
use crate::buildah::b::{
    blob_path, buildah_dockerconatinerfile_build, buildah_push_to_oci_layout,
    buildah_unshare_build, pathbuf_to_actionable_buildah_path, read_oci_index_manifest_digest,
};
use crate::clilib::cliargs::{Args, WA};
use crate::oci::oci_helpers::{digest_hex, is_digest_reference, oci_error, sha256_digest};
use actix_web::{http::StatusCode, web, App, HttpResponse, HttpServer};
use std::io;
use std::time::{Duration, SystemTime};
pub mod buildah;
pub mod clilib;
pub mod oci;

/// Send-safe error type for use inside web::block() closures.
/// HttpResponse is !Send, so we carry the error data and convert to HttpResponse after.
struct RegistryError {
    status: u16,
    code: String,
    message: String,
}

impl RegistryError {
    fn new(status: u16, code: &str, message: &str) -> Self {
        RegistryError {
            status,
            code: code.to_string(),
            message: message.to_string(),
        }
    }

    fn into_response(self) -> HttpResponse {
        oci_error(
            StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            &self.code,
            &self.message,
        )
    }
}

#[actix_web::main]
async fn main() -> std::result::Result<(), io::Error> {
    let arg = Args::args_or_exit();
    let wa: web::Data<WA> = arg.args_to_data_wa();
    HttpServer::new(move || {
        App::new()
            .app_data(wa.to_owned())
            // Pull endpoints (required by OCI spec)
            .service(web::resource("/v2/").route(web::get().to(v2_base)))
            .service(
                web::resource("/v2/{name:.*}/manifests/{reference}")
                    .route(web::get().to(get_manifest))
                    .route(web::head().to(head_manifest))
                    .route(web::put().to(unsupported))
                    .route(web::delete().to(unsupported)),
            )
            .service(
                web::resource("/v2/{name:.*}/blobs/uploads/{reference}")
                    .route(web::patch().to(unsupported))
                    .route(web::put().to(unsupported))
                    .route(web::delete().to(unsupported))
                    .route(web::get().to(unsupported)),
            )
            .service(
                web::resource("/v2/{name:.*}/blobs/uploads/")
                    .route(web::post().to(unsupported)),
            )
            .service(
                web::resource("/v2/{name:.*}/blobs/{digest}")
                    .route(web::get().to(get_blob))
                    .route(web::head().to(head_blob))
                    .route(web::delete().to(unsupported)),
            )
    })
    .bind((arg.bind_addr, arg.bind_port))?
    .client_request_timeout(Duration::from_secs(600))
    .keep_alive(Duration::from_secs(600))
    .run()
    .await
}

/// GET /v2/ — OCI spec base endpoint (end-1).
async fn v2_base() -> HttpResponse {
    HttpResponse::Ok()
        .insert_header(("Docker-Distribution-API-Version", "registry/2.0"))
        .json(serde_json::json!({}))
}

/// Returns true if the OCI layout cache for this image was written within the last 12 hours.
fn is_cache_fresh(name: &str, data: &WA) -> bool {
    let index = data.oci_cache_dir.join(name).join("index.json");
    std::fs::metadata(&index)
        .and_then(|m| m.modified())
        .map(|mtime| {
            SystemTime::now()
                .duration_since(mtime)
                .unwrap_or(Duration::MAX)
                < Duration::from_secs(12 * 3600)
        })
        .unwrap_or(false)
}

/// Reads manifest from existing OCI layout cache (no build).
fn serve_from_cache(name: &str, data: &WA) -> Result<(Vec<u8>, String), RegistryError> {
    let manifest_digest = read_oci_index_manifest_digest(&data.oci_cache_dir, name)
        .map_err(|e| RegistryError::new(500, "MANIFEST_UNKNOWN", &e.to_string()))?;

    let hex = digest_hex(&manifest_digest)
        .ok_or_else(|| RegistryError::new(500, "MANIFEST_UNKNOWN", "invalid digest in index.json"))?;

    let manifest_path = blob_path(&data.oci_cache_dir, name, hex);
    let manifest_bytes = std::fs::read(&manifest_path)
        .map_err(|e| RegistryError::new(500, "MANIFEST_UNKNOWN", &e.to_string()))?;

    let computed_digest = sha256_digest(&manifest_bytes);
    Ok((manifest_bytes, computed_digest))
}

/// Builds an image by name, exports to OCI layout, returns manifest bytes + digest.
fn build_and_export(name: &str, data: &WA) -> Result<(Vec<u8>, String), RegistryError> {
    let (buildah_script_opt, somefile_opt) =
        pathbuf_to_actionable_buildah_path(&data.con_dir_path, name).map_err(|_| {
            RegistryError::new(404, "NAME_UNKNOWN", "repository name not known to registry")
        })?;

    let mut hash = String::new();
    if let Some(ref x) = buildah_script_opt {
        hash = buildah_unshare_build(x)
            .map_err(|e| RegistryError::new(500, "MANIFEST_UNKNOWN", &e.to_string()))?;
    }
    if let Some(ref x) = somefile_opt {
        hash = buildah_dockerconatinerfile_build(x)
            .map_err(|e| RegistryError::new(500, "MANIFEST_UNKNOWN", &e.to_string()))?;
    }

    if hash.is_empty() {
        return Err(RegistryError::new(
            404,
            "MANIFEST_UNKNOWN",
            "no buildable content found",
        ));
    }

    // Export to OCI layout
    buildah_push_to_oci_layout(&hash, &data.oci_cache_dir, name)
        .map_err(|e| RegistryError::new(500, "MANIFEST_UNKNOWN", &e.to_string()))?;

    // Serve from the freshly-exported cache
    serve_from_cache(name, data)
}

/// Resolves a manifest by reference (tag or digest). Returns (bytes, digest).
fn resolve_manifest(
    name: &str,
    reference: &str,
    data: &WA,
) -> Result<(Vec<u8>, String), RegistryError> {
    if is_digest_reference(reference) {
        // Digest-based lookup: find in cache
        let hex = digest_hex(reference)
            .ok_or_else(|| RegistryError::new(400, "DIGEST_INVALID", "invalid digest format"))?;
        let path = blob_path(&data.oci_cache_dir, name, hex);
        let bytes = std::fs::read(&path)
            .map_err(|_| RegistryError::new(404, "MANIFEST_UNKNOWN", "manifest unknown to registry"))?;
        let computed = sha256_digest(&bytes);
        Ok((bytes, computed))
    } else {
        // Tag-based: serve from cache if fresh, otherwise build
        if is_cache_fresh(name, data) {
            serve_from_cache(name, data)
        } else {
            build_and_export(name, data)
        }
    }
}

/// GET /v2/{name}/manifests/{reference} — Pull manifest (end-3).
async fn get_manifest(
    path: web::Path<(String, String)>,
    data: web::Data<WA>,
) -> HttpResponse {
    let (name, reference) = path.into_inner();
    let data_ref = data.into_inner();
    let result = web::block(move || resolve_manifest(&name, &reference, &data_ref)).await;
    match result {
        Ok(Ok((bytes, digest))) => {
            let content_type = detect_manifest_content_type(&bytes);
            HttpResponse::Ok()
                .insert_header(("Content-Type", content_type))
                .insert_header(("Docker-Content-Digest", digest.as_str()))
                .insert_header(("Docker-Distribution-API-Version", "registry/2.0"))
                .body(bytes)
        }
        Ok(Err(e)) => e.into_response(),
        Err(_) => oci_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "MANIFEST_UNKNOWN",
            "internal error during build",
        ),
    }
}

/// HEAD /v2/{name}/manifests/{reference} — Check manifest existence (end-3).
async fn head_manifest(
    path: web::Path<(String, String)>,
    data: web::Data<WA>,
) -> HttpResponse {
    let (name, reference) = path.into_inner();
    let data_ref = data.into_inner();
    let result = web::block(move || resolve_manifest(&name, &reference, &data_ref)).await;
    match result {
        Ok(Ok((bytes, digest))) => {
            let content_type = detect_manifest_content_type(&bytes);
            HttpResponse::Ok()
                .insert_header(("Content-Type", content_type))
                .insert_header(("Docker-Content-Digest", digest.as_str()))
                .insert_header(("Docker-Distribution-API-Version", "registry/2.0"))
                .insert_header(("Content-Length", bytes.len().to_string()))
                .finish()
        }
        Ok(Err(e)) => e.into_response(),
        Err(_) => oci_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "MANIFEST_UNKNOWN",
            "internal error during build",
        ),
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
                StatusCode::BAD_REQUEST,
                "DIGEST_INVALID",
                "invalid digest format",
            );
        }
    };

    let data_ref = data.into_inner();
    let digest_clone = digest.clone();
    let result = web::block(move || {
        let path = blob_path(&data_ref.oci_cache_dir, &name, &hex);
        std::fs::read(&path).map_err(|_| RegistryError::new(404, "BLOB_UNKNOWN", "blob unknown to registry"))
    })
    .await;

    match result {
        Ok(Ok(bytes)) => HttpResponse::Ok()
            .insert_header(("Content-Type", "application/octet-stream"))
            .insert_header(("Docker-Content-Digest", digest_clone.as_str()))
            .insert_header(("Content-Length", bytes.len().to_string()))
            .body(bytes),
        Ok(Err(e)) => e.into_response(),
        Err(_) => oci_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "BLOB_UNKNOWN",
            "internal error reading blob",
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
                StatusCode::BAD_REQUEST,
                "DIGEST_INVALID",
                "invalid digest format",
            );
        }
    };

    let data_ref = data.into_inner();
    let digest_clone = digest.clone();
    let result = web::block(move || {
        let path = blob_path(&data_ref.oci_cache_dir, &name, &hex);
        std::fs::metadata(&path).map_err(|_| RegistryError::new(404, "BLOB_UNKNOWN", "blob unknown to registry"))
    })
    .await;

    match result {
        Ok(Ok(meta)) => HttpResponse::Ok()
            .insert_header(("Content-Type", "application/octet-stream"))
            .insert_header(("Docker-Content-Digest", digest_clone.as_str()))
            .insert_header(("Content-Length", meta.len().to_string()))
            .finish(),
        Ok(Err(e)) => e.into_response(),
        Err(_) => oci_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "BLOB_UNKNOWN",
            "internal error checking blob",
        ),
    }
}

/// Returns 405 Method Not Allowed with OCI UNSUPPORTED error for read-only operations.
async fn unsupported() -> HttpResponse {
    oci_error(
        StatusCode::METHOD_NOT_ALLOWED,
        "UNSUPPORTED",
        "this registry is read-only",
    )
}
