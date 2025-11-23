use std::env;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    #[error("{0} must be set")]
    Missing(&'static str),
    #[error("{name} is invalid: {reason}")]
    Invalid { name: &'static str, reason: String },
}

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub port: u16,
    pub token_ttl_hours: i64,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|name| env::var(name).ok())
    }

    /// Reads the configuration through `lookup`, which makes it testable
    /// without touching the process environment.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let get = |name: &str| lookup(name).filter(|v| !v.trim().is_empty());

        let database_url = get("DATABASE_URL").ok_or(ConfigError::Missing("DATABASE_URL"))?;

        let jwt_secret = get("JWT_SECRET").ok_or(ConfigError::Missing("JWT_SECRET"))?;
        let port = match get("PORT") {
            Some(v) => v.parse().map_err(|_| ConfigError::Invalid {
                name: "PORT",
                reason: format!("{v:?} is not a port number"),
            })?,
            None => 4407,
        };

        let token_ttl_hours = match get("TOKEN_TTL_HOURS") {
            Some(v) => match v.parse::<i64>() {
                Ok(h) if h > 0 => h,
                _ => {
                    return Err(ConfigError::Invalid {
                        name: "TOKEN_TTL_HOURS",
                        reason: format!("{v:?} is not a positive number of hours"),
                    });
                }
            },
            None => 24 * 7,
        };

        Ok(Self {
            database_url,
            jwt_secret,
            port,
            token_ttl_hours,
        })
    }
}
