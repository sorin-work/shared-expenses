use std::sync::Arc;

use sqlx::PgPool;

use crate::auth::token::TokenKeys;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub tokens: Arc<TokenKeys>,
}

impl AppState {
    pub fn new(pool: PgPool, tokens: TokenKeys) -> Self {
        Self {
            pool,
            tokens: Arc::new(tokens),
        }
    }
}
