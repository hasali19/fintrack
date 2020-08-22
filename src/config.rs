use std::env;

use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

pub struct Config {
    pub http_address: String,
    pub http_port: u16,
    pub secret_key: Vec<u8>,
    pub db_username: String,
    pub db_password: String,
    pub db_hostname: String,
    pub db_port: u16,
    pub db_name: String,
}

impl Config {
    pub fn from_env() -> Config {
        Config {
            http_address: var_or_str("FINTRACK_HTTP_ADDRESS", "127.0.0.1"),
            http_port: var_or_str("FINTRACK_HTTP_PORT", "8000").parse().unwrap(),
            secret_key: env::var("FINTRACK_SECRET_KEY").unwrap().into_bytes(),
            db_username: var_or_str("FINTRACK_DB_USERNAME", "postgres"),
            db_password: env::var("FINTRACK_DB_PASSWORD").unwrap(),
            db_hostname: var_or_str("FINTRACK_DB_HOSTNAME", "localhost"),
            db_port: var_or_str("FINTRACK_DB_PORT", "5432").parse().unwrap(),
            db_name: var_or_str("FINTRACK_DB_NAME", "fintrack"),
        }
    }

    pub fn db_url(&self) -> String {
        format!(
            "postgresql://{}:{}@{}:{}/{}",
            self.db_username,
            utf8_percent_encode(&self.db_password, NON_ALPHANUMERIC),
            self.db_hostname,
            self.db_port,
            self.db_name
        )
    }
}

fn var_or_str(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| default.to_owned())
}
