use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub iat: i64,
    pub exp: i64,
}

pub struct TokenKeys {
    encoding: EncodingKey,
    decoding: DecodingKey,
    ttl: Duration,
}

impl TokenKeys {
    pub fn new(secret: &str, ttl_hours: i64) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret.as_bytes()),
            decoding: DecodingKey::from_secret(secret.as_bytes()),
            ttl: Duration::hours(ttl_hours),
        }
    }

    pub fn issue(&self, user_id: Uuid) -> Result<String, AppError> {
        let now = Utc::now();
        let claims = Claims {
            sub: user_id,
            iat: now.timestamp(),
            exp: (now + self.ttl).timestamp(),
        };
        encode(&Header::new(Algorithm::HS256), &claims, &self.encoding)
            .map_err(|e| AppError::Internal(format!("signing token: {e}")))
    }

    /// Returns the user id from a valid, unexpired token.
    pub fn verify(&self, token: &str) -> Result<Uuid, AppError> {
        let validation = Validation::new(Algorithm::HS256);
        decode::<Claims>(token, &self.decoding, &validation)
            .map(|data| data.claims.sub)
            .map_err(|_| AppError::Unauthorized)
    }
}
