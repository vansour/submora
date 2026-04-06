use std::{
    collections::HashMap,
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    str::FromStr,
};

use axum::http::HeaderValue;

use crate::core::{is_valid_password_length, is_valid_username};

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub host: IpAddr,
    pub port: u16,
    pub web_dist_dir: PathBuf,
    pub database_url: String,
    pub cookie_secure: bool,
    pub session_ttl_minutes: i64,
    pub session_cleanup_interval_secs: u64,
    pub trust_proxy_headers: bool,
    pub login_max_attempts: usize,
    pub login_window_secs: u64,
    pub login_lockout_secs: u64,
    pub public_max_requests: usize,
    pub public_window_secs: u64,
    pub db_max_connections: u32,
    pub fetch_timeout_secs: u64,
    pub fetch_host_overrides: HashMap<String, Vec<SocketAddr>>,
    pub concurrent_limit: usize,
    pub max_links_per_user: usize,
    pub max_users: usize,
    pub admin_user: String,
    pub admin_password: String,
    pub cors_allow_origin: Vec<String>,
}

impl ServerConfig {
    pub fn from_env() -> Result<Self, String> {
        let config = Self {
            host: parse_env_or_default("HOST", IpAddr::V4(Ipv4Addr::UNSPECIFIED))?,
            port: parse_env_or_default("PORT", 8080_u16)?,
            web_dist_dir: env::var("WEB_DIST_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("dist")),
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://data/substore.db?mode=rwc".to_string()),
            cookie_secure: parse_env_or_default("COOKIE_SECURE", false)?,
            session_ttl_minutes: parse_env_or_default("SESSION_TTL_MINUTES", 60_i64 * 24 * 7)?,
            session_cleanup_interval_secs: parse_env_or_default(
                "SESSION_CLEANUP_INTERVAL_SECS",
                300_u64,
            )?,
            trust_proxy_headers: parse_env_or_default("TRUST_PROXY_HEADERS", false)?,
            login_max_attempts: parse_env_or_default("LOGIN_MAX_ATTEMPTS", 5_usize)?,
            login_window_secs: parse_env_or_default("LOGIN_WINDOW_SECS", 300_u64)?,
            login_lockout_secs: parse_env_or_default("LOGIN_LOCKOUT_SECS", 900_u64)?,
            public_max_requests: parse_env_or_default("PUBLIC_MAX_REQUESTS", 60_usize)?,
            public_window_secs: parse_env_or_default("PUBLIC_WINDOW_SECS", 60_u64)?,
            db_max_connections: parse_env_or_default("DB_MAX_CONNECTIONS", 5_u32)?,
            fetch_timeout_secs: parse_env_or_default("FETCH_TIMEOUT_SECS", 10_u64)?,
            fetch_host_overrides: match env::var("FETCH_HOST_OVERRIDES") {
                Ok(value) => parse_fetch_host_overrides(&value)?,
                Err(_) => HashMap::new(),
            },
            concurrent_limit: parse_env_or_default("CONCURRENT_LIMIT", 10_usize)?,
            max_links_per_user: parse_env_or_default("MAX_LINKS_PER_USER", 100_usize)?,
            max_users: parse_env_or_default("MAX_USERS", 100_usize)?,
            admin_user: env::var("ADMIN_USER").unwrap_or_else(|_| "admin".to_string()),
            admin_password: env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "admin".to_string()),
            cors_allow_origin: env::var("CORS_ALLOW_ORIGIN")
                .unwrap_or_else(|_| "http://127.0.0.1:8081,http://localhost:8081".to_string())
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
        };

        config.validate()?;
        Ok(config)
    }

    pub fn socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.host, self.port)
    }

    fn validate(&self) -> Result<(), String> {
        validate_positive("PORT", self.port)?;
        validate_positive("SESSION_TTL_MINUTES", self.session_ttl_minutes)?;
        validate_positive(
            "SESSION_CLEANUP_INTERVAL_SECS",
            self.session_cleanup_interval_secs,
        )?;
        validate_positive("LOGIN_MAX_ATTEMPTS", self.login_max_attempts)?;
        validate_positive("LOGIN_WINDOW_SECS", self.login_window_secs)?;
        validate_positive("LOGIN_LOCKOUT_SECS", self.login_lockout_secs)?;
        validate_positive("PUBLIC_MAX_REQUESTS", self.public_max_requests)?;
        validate_positive("PUBLIC_WINDOW_SECS", self.public_window_secs)?;
        validate_positive("DB_MAX_CONNECTIONS", self.db_max_connections)?;
        validate_positive("FETCH_TIMEOUT_SECS", self.fetch_timeout_secs)?;
        validate_positive("CONCURRENT_LIMIT", self.concurrent_limit)?;
        validate_positive("MAX_LINKS_PER_USER", self.max_links_per_user)?;
        validate_positive("MAX_USERS", self.max_users)?;

        if self.database_url.trim().is_empty() {
            return Err("DATABASE_URL must not be empty".to_string());
        }

        if !is_valid_username(&self.admin_user) {
            return Err("ADMIN_USER must be a valid username".to_string());
        }

        if !is_valid_password_length(&self.admin_password) {
            return Err("ADMIN_PASSWORD must be 1-128 characters".to_string());
        }

        for origin in &self.cors_allow_origin {
            HeaderValue::from_str(origin).map_err(|error| {
                format!("CORS_ALLOW_ORIGIN contains invalid origin {origin:?}: {error}")
            })?;
        }

        Ok(())
    }
}

fn parse_env_or_default<T>(name: &str, default: T) -> Result<T, String>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    match env::var(name) {
        Ok(value) => value
            .parse::<T>()
            .map_err(|error| format!("{name} has invalid value {value:?}: {error}")),
        Err(_) => Ok(default),
    }
}

fn validate_positive<T>(name: &str, value: T) -> Result<(), String>
where
    T: PartialEq + From<u8>,
{
    if value == T::from(0) {
        Err(format!("{name} must be greater than 0"))
    } else {
        Ok(())
    }
}

fn parse_fetch_host_overrides(input: &str) -> Result<HashMap<String, Vec<SocketAddr>>, String> {
    let mut overrides = HashMap::new();

    for entry in input
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        let (host, addrs) = entry.split_once('=').ok_or_else(|| {
            format!("FETCH_HOST_OVERRIDES entry {entry:?} must use host:port=addr|addr syntax")
        })?;
        let host = host.trim();
        if host.is_empty() {
            return Err("FETCH_HOST_OVERRIDES host key must not be empty".to_string());
        }

        let mut resolved_addrs = Vec::new();
        for raw_addr in addrs
            .split('|')
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            let addr = SocketAddr::from_str(raw_addr).map_err(|error| {
                format!(
                    "FETCH_HOST_OVERRIDES entry {entry:?} contains invalid socket address {raw_addr:?}: {error}"
                )
            })?;
            resolved_addrs.push(addr);
        }

        if resolved_addrs.is_empty() {
            return Err(format!(
                "FETCH_HOST_OVERRIDES entry {entry:?} must include at least one socket address"
            ));
        }

        overrides.insert(host.to_string(), resolved_addrs);
    }

    Ok(overrides)
}
