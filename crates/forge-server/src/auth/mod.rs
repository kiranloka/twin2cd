mod middleware;

pub use middleware::{AuthContext, require_auth, require_permission};

use sha2::{Digest, Sha256};

pub fn hash_token(raw: &str) -> String {
    let digest = Sha256::digest(raw.as_bytes());
    hex::encode(digest)
}
