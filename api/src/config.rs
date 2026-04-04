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
    pub seed_demo: bool,
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

        let seed_demo = match get("SEED_DEMO").as_deref() {
            None => false,
            Some(v) => parse_bool(v).ok_or_else(|| ConfigError::Invalid {
                name: "SEED_DEMO",
                reason: format!("{v:?} is not true or false"),
            })?,
        };

        Ok(Self {
            database_url,
            jwt_secret,
            port,
            token_ttl_hours,
            seed_demo,
        })
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    const SECRET: &str = "0123456789abcdef0123456789abcdef";

    fn load(vars: &[(&str, &str)]) -> Result<Config, ConfigError> {
        let map: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Config::from_lookup(|name| map.get(name).cloned())
    }

    #[test]
    fn uses_defaults() {
        let config = load(&[("DATABASE_URL", "postgres://x"), ("JWT_SECRET", SECRET)]).unwrap();
        assert_eq!(config.port, 4407);
        assert_eq!(config.token_ttl_hours, 168);
        assert!(!config.seed_demo);
    }

    #[test]
    fn requires_database_url_and_secret() {
        assert_eq!(
            load(&[("JWT_SECRET", SECRET)]).unwrap_err(),
            ConfigError::Missing("DATABASE_URL")
        );
        assert_eq!(
            load(&[("DATABASE_URL", "postgres://x"), ("JWT_SECRET", "  ")]).unwrap_err(),
            ConfigError::Missing("JWT_SECRET")
        );
    }

    #[test]
    fn parses_overrides() {
        let config = load(&[
            ("DATABASE_URL", "postgres://x"),
            ("JWT_SECRET", SECRET),
            ("PORT", "8080"),
            ("TOKEN_TTL_HOURS", "12"),
            ("SEED_DEMO", "TRUE"),
        ])
        .unwrap();
        assert_eq!(config.port, 8080);
        assert_eq!(config.token_ttl_hours, 12);
        assert!(config.seed_demo);
    }

    #[test]
    fn rejects_bad_values() {
        let base = [("DATABASE_URL", "postgres://x"), ("JWT_SECRET", SECRET)];
        for (name, value) in [
            ("PORT", "http"),
            ("TOKEN_TTL_HOURS", "0"),
            ("SEED_DEMO", "maybe"),
        ] {
            let mut vars = base.to_vec();
            vars.push((name, value));
            assert!(load(&vars).is_err(), "{name}={value} should fail");
        }
    }
}
