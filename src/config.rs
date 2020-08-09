use std::env;

pub struct Config {
    pub secret_key: Vec<u8>,
}

impl Config {
    pub fn from_env() -> Config {
        Config {
            secret_key: env::var("FINTRACK_SECRET_KEY").unwrap().into_bytes(),
        }
    }
}
