use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};

use crate::error::AppError;

/// Hashes a password with argon2id and a random salt (PHC string format).
pub fn hash(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::Internal(format!("hashing password: {e}")))
}

/// Checks a password against a stored hash. A malformed hash counts as a
/// mismatch.
pub fn verify(password: &str, stored: &str) -> bool {
    PasswordHash::new(stored)
        .map(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        })
        .unwrap_or(false)
}

/// Runs the CPU heavy hashing on the blocking thread pool.
pub async fn hash_async(password: String) -> Result<String, AppError> {
    tokio::task::spawn_blocking(move || hash(&password))
        .await
        .map_err(|e| AppError::Internal(format!("hash task: {e}")))?
}

pub async fn verify_async(password: String, stored: String) -> Result<bool, AppError> {
    tokio::task::spawn_blocking(move || verify(&password, &stored))
        .await
        .map_err(|e| AppError::Internal(format!("verify task: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_verify_against_the_original_password() {
        let stored = hash("correct horse battery staple").unwrap();
        assert!(stored.starts_with("$argon2id$"));
        assert!(verify("correct horse battery staple", &stored));
        assert!(!verify("Correct horse battery staple", &stored));
    }

    #[test]
    fn same_password_gets_a_different_salt() {
        assert_ne!(hash("password1").unwrap(), hash("password1").unwrap());
    }

    #[test]
    fn malformed_hashes_never_verify() {
        assert!(!verify("anything", "not a hash"));
        assert!(!verify("anything", ""));
    }
}
