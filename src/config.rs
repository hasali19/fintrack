use std::env;

pub struct Config {
    pub http_address: String,
    pub http_port: u16,
    pub secret_key: Vec<u8>,
    pub db_url: String,
}

impl Config {
    pub fn from_env() -> Config {
        Config {
            http_address: var_or_str("FINTRACK_HTTP_ADDRESS", "127.0.0.1"),
            http_port: var_or_str("FINTRACK_HTTP_PORT", "8000").parse().unwrap(),
            secret_key: env::var("FINTRACK_SECRET_KEY").unwrap().into_bytes(),
            db_url: env::var("DATABASE_URL").unwrap(),
        }
    }
}

fn var_or_str(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| default.to_owned())
}
