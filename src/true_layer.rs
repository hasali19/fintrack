use std::env;
use std::fmt::{self, Display};

use anyhow::anyhow;
use serde::Deserialize;

pub struct Client {
    config: TrueLayerConfig,
}

pub struct TrueLayerConfig {
    client_id: String,
    client_secret: String,
    auth_link: String,
}

impl TrueLayerConfig {
    pub fn from_env() -> TrueLayerConfig {
        TrueLayerConfig {
            client_id: env::var("TRUE_LAYER_CLIENT_ID").unwrap(),
            client_secret: env::var("TRUE_LAYER_CLIENT_SECRET").unwrap(),
            auth_link: env::var("TRUE_LAYER_AUTH_LINK").unwrap(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ErrorResponse {
    error: String,
    error_description: Option<String>,
}

impl Display for ErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(f)
    }
}

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: i64,
    pub refresh_token: String,
    pub token_type: String,
}

impl Client {
    pub fn new() -> Client {
        Client {
            config: TrueLayerConfig::from_env(),
        }
    }

    pub fn auth_link(&self) -> &str {
        self.config.auth_link.as_str()
    }

    pub async fn exchange_code(&self, code: &str) -> anyhow::Result<TokenResponse> {
        // TODO: This url should be read from env
        let mut res = surf::post("https://auth.truelayer-sandbox.com/connect/token")
            .body_form(&serde_json::json!({
                "client_id": self.config.client_id,
                "client_secret": self.config.client_secret,
                "code": code,
                "grant_type": "authorization_code",
                "redirect_uri": "http://localhost:8000/callback",
            }))?
            .await
            .map_err(|e| anyhow!(e))?;

        if res.status() != surf::http::StatusCode::OK {
            return tl_error(res).await;
        }

        Ok(res.body_json().await?)
    }
}

async fn tl_error<T>(mut res: surf::Response) -> anyhow::Result<T> {
    let status = res.status();

    if status.is_client_error() {
        let res: ErrorResponse = res.body_json().await?;
        return Err(anyhow!(res));
    }

    Err(anyhow!(
        "request to TrueLayer failed with status: {}",
        status
    ))
}
