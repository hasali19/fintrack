use std::env;
use std::fmt::{self, Display};

use anyhow::anyhow;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use surf::http::StatusCode;

#[async_trait]
pub trait AuthProvider {
    async fn access_token(&self, provider: &str, true_layer: &Client) -> anyhow::Result<String>;
}

pub struct Client {
    config: TrueLayerConfig,
    auth_provider: Box<dyn AuthProvider + Send + Sync>,
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

#[derive(Debug, Deserialize)]
pub struct Provider {
    pub provider_id: String,
    pub display_name: String,
    pub logo_url: String,
}

#[derive(Deserialize)]
struct Results<T> {
    results: Vec<T>,
}

#[derive(Debug, Deserialize)]
pub struct TokenMetadata {
    pub provider: ProviderMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderMetadata {
    pub provider_id: String,
    pub display_name: String,
    pub logo_uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Account {
    account_id: String,
    account_type: String,
    account_number: AccountNumber,
    currency: String,
    display_name: String,
    update_timestamp: String,
    provider: ProviderMetadata,
    description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountNumber {
    iban: Option<String>,
    number: Option<String>,
    sort_code: Option<String>,
}

impl Client {
    pub fn new(auth_provider: impl AuthProvider + Send + Sync + 'static) -> Client {
        Client {
            config: TrueLayerConfig::from_env(),
            auth_provider: Box::new(auth_provider),
        }
    }

    pub fn auth_link(&self, callback: &str) -> String {
        format!("{}&redirect_uri={}", self.config.auth_link, callback)
    }

    pub async fn exchange_code(&self, code: &str, callback: &str) -> anyhow::Result<TokenResponse> {
        let mut res = surf::post("https://auth.truelayer-sandbox.com/connect/token")
            .body_form(&serde_json::json!({
                "client_id": self.config.client_id,
                "client_secret": self.config.client_secret,
                "code": code,
                "grant_type": "authorization_code",
                "redirect_uri": callback,
            }))?
            .await
            .map_err(|e| anyhow!(e))?;

        if res.status() != StatusCode::OK {
            return tl_error(res).await;
        }

        Ok(res.body_json().await?)
    }

    pub async fn renew_token(&self, refresh_token: &str) -> anyhow::Result<TokenResponse> {
        let mut res = surf::post("https://auth.truelayer-sandbox.com/connect/token")
            .body_form(&serde_json::json!({
                "client_id": self.config.client_id,
                "client_secret": self.config.client_secret,
                "refresh_token": refresh_token,
                "grant_type": "refresh_token",
            }))?
            .await
            .map_err(|e| anyhow!(e))?;

        if res.status() != StatusCode::OK {
            return tl_error(res).await;
        }

        Ok(res.body_json().await?)
    }

    pub async fn token_metadata(&self, access_token: &str) -> anyhow::Result<TokenMetadata> {
        let mut res = surf::get("https://api.truelayer-sandbox.com/data/v1/me")
            .set_header("Authorization", &format!("Bearer {}", access_token))
            .await
            .map_err(|e| anyhow!(e))?;

        if res.status() != StatusCode::OK {
            return tl_error(res).await;
        }

        Ok(res
            .body_json::<Results<_>>()
            .await?
            .results
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("invalid metadata response"))?)
    }

    pub async fn supported_providers(&self) -> anyhow::Result<Vec<Provider>> {
        Ok(
            surf::get("https://auth.truelayer-sandbox.com/api/providers")
                .recv_json()
                .await
                .map_err(|e| anyhow!(e))?,
        )
    }

    pub async fn accounts(&self, provider: &str) -> anyhow::Result<Vec<Account>> {
        let access_token = self.auth_provider.access_token(provider, &self).await?;
        Ok(
            surf::get("https://api.truelayer-sandbox.com/data/v1/accounts")
                .set_header("Authorization", format!("Bearer {}", access_token))
                .recv_json::<Results<_>>()
                .await
                .map_err(|e| anyhow!(e))?
                .results,
        )
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
