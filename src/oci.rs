pub mod oci_helpers {
    use actix_web::HttpResponse;
    use sha2::{Digest, Sha256};

    /// Computes sha256 digest of bytes, returning "sha256:{hex}" string.
    pub fn sha256_digest(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        format!("sha256:{}", hex::encode(result))
    }

    /// Returns true if the reference looks like a digest (e.g. "sha256:abc123...").
    pub fn is_digest_reference(reference: &str) -> bool {
        reference.starts_with("sha256:")
            && reference.len() > 7
            && reference[7..].chars().all(|c| c.is_ascii_hexdigit())
    }

    /// Extracts the hex portion from a digest string like "sha256:abc123".
    pub fn digest_hex(digest: &str) -> Option<&str> {
        digest.strip_prefix("sha256:")
    }

    /// Returns an OCI-formatted error response.
    pub fn oci_error(status: actix_web::http::StatusCode, code: &str, message: &str) -> HttpResponse {
        HttpResponse::build(status).json(serde_json::json!({
            "errors": [{
                "code": code,
                "message": message
            }]
        }))
    }
}
