use std::env;
use std::fmt::{self, Display};

use anyhow::anyhow;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::{header, StatusCode};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[async_trait]
pub trait AuthProvider {
    async fn token_for_provider(
        &self,
        true_layer: &Client,
        provider_id: &str,
    ) -> anyhow::Result<String>;

    async fn token_for_account(
        &self,
        true_layer: &Client,
        account_id: &str,
    ) -> anyhow::Result<String>;
}

pub enum Env {
    Sandbox,
    Live,
}

pub struct Client {
    client: reqwest::Client,
    config: TrueLayerConfig,
    auth_provider: Box<dyn AuthProvider + Send + Sync>,
}

pub struct TrueLayerConfig {
    client_id: String,
    client_secret: String,
    env: Env,
}

impl TrueLayerConfig {
    pub fn from_env() -> TrueLayerConfig {
        TrueLayerConfig {
            client_id: env::var("TRUE_LAYER_CLIENT_ID").unwrap(),
            client_secret: env::var("TRUE_LAYER_CLIENT_SECRET").unwrap(),
            env: env::var("TRUE_LAYER_ENV")
                .map(|v| match v.to_lowercase().as_str() {
                    "sandbox" => Env::Sandbox,
                    "live" => Env::Live,
                    _ => panic!("invalid value for `TRUE_LAYER_ENV`"),
                })
                .unwrap_or(Env::Sandbox),
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
    pub account_id: String,
    pub account_type: String,
    pub account_number: AccountNumber,
    pub currency: String,
    pub display_name: String,
    pub update_timestamp: String,
    pub provider: ProviderMetadata,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountNumber {
    pub iban: Option<String>,
    pub number: Option<String>,
    pub sort_code: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountBalance {
    pub currency: String,
    pub available: f64,
    pub current: f64,
    pub overdraft: f64,
    pub update_timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub transaction_id: String,
    pub timestamp: DateTime<Utc>,
    pub description: String,
    pub transaction_type: String,
    pub transaction_category: String,
    pub transaction_classification: Vec<String>,
    pub merchant_name: Option<String>,
    pub amount: Decimal,
    pub currency: String,
    pub meta: Value,
    pub running_balance: TransactionRunningBalance,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionRunningBalance {
    pub amount: Option<f64>,
    pub currency: Option<String>,
}

impl Client {
    pub fn new(auth_provider: impl AuthProvider + Send + Sync + 'static) -> Client {
        Client {
            client: reqwest::Client::new(),
            config: TrueLayerConfig::from_env(),
            auth_provider: Box::new(auth_provider),
        }
    }

    fn hostname(&self) -> &'static str {
        match self.config.env {
            Env::Sandbox => "truelayer-sandbox.com",
            Env::Live => "truelayer.com",
        }
    }

    pub fn auth_link(&self, callback: &str) -> String {
        let providers = match self.config.env {
            Env::Sandbox => "uk-ob-all%20uk-oauth-all%20uk-cs-mock",
            Env::Live => "uk-ob-all%20uk-oauth-all",
        };

        format!(
            "https://auth.{}/?\
                response_type=code&\
                client_id={}&\
                scope=info%20accounts%20balance%20cards%20transactions%20direct_debits%20standing_orders%20offline_access&\
                redirect_uri={}&\
                providers={}",
            self.hostname(),
            self.config.client_id,
            callback,
            providers,
        )
    }

    pub async fn exchange_code(&self, code: &str, callback: &str) -> anyhow::Result<TokenResponse> {
        let url = format!("https://auth.{}/connect/token", self.hostname());
        let res = self
            .client
            .post(&url)
            .form(&serde_json::json!({
                "client_id": self.config.client_id,
                "client_secret": self.config.client_secret,
                "code": code,
                "grant_type": "authorization_code",
                "redirect_uri": callback,
            }))
            .send()
            .await?;

        if res.status() != StatusCode::OK {
            return tl_error(res).await;
        }

        Ok(res.json().await?)
    }

    pub async fn renew_token(&self, refresh_token: &str) -> anyhow::Result<TokenResponse> {
        let url = format!("https://auth.{}/connect/token", self.hostname());
        let res = self
            .client
            .post(&url)
            .form(&serde_json::json!({
                "client_id": self.config.client_id,
                "client_secret": self.config.client_secret,
                "refresh_token": refresh_token,
                "grant_type": "refresh_token",
            }))
            .send()
            .await?;

        if res.status() != StatusCode::OK {
            return tl_error(res).await;
        }

        Ok(res.json().await?)
    }

    pub async fn token_metadata(&self, access_token: &str) -> anyhow::Result<TokenMetadata> {
        let url = format!("https://api.{}/data/v1/me", self.hostname());
        let res = self
            .client
            .get(&url)
            .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
            .send()
            .await?;

        if res.status() != StatusCode::OK {
            return tl_error(res).await;
        }

        Ok(res
            .json::<Results<_>>()
            .await?
            .results
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("invalid metadata response"))?)
    }

    pub async fn supported_providers(&self) -> anyhow::Result<Vec<Provider>> {
        let url = format!("https://auth.{}/api/providers", self.hostname());
        Ok(self.client.get(&url).send().await?.json().await?)
    }

    pub async fn accounts(&self, provider: &str) -> anyhow::Result<Vec<Account>> {
        let access_token = self
            .auth_provider
            .token_for_provider(&self, provider)
            .await?;

        let url = format!("https://api.{}/data/v1/accounts", self.hostname());

        Ok(self
            .client
            .get(&url)
            .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
            .send()
            .await?
            .json::<Results<_>>()
            .await?
            .results)
    }

    pub async fn account_balance(&self, account: &str) -> anyhow::Result<AccountBalance> {
        let access_token = self.auth_provider.token_for_account(&self, account).await?;
        let url = format!(
            "https://api.{}/data/v1/accounts/{}/balance",
            self.hostname(),
            account
        );

        let res = self
            .client
            .get(&url)
            .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
            .send()
            .await?;

        if !res.status().is_success() {
            return tl_error(res).await;
        }

        Ok(res
            .json::<Results<_>>()
            .await?
            .results
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("invalid account balance response"))?)
    }

    pub async fn transactions(
        &self,
        account: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> anyhow::Result<Vec<Transaction>> {
        let access_token = self.auth_provider.token_for_account(&self, account).await?;
        let url = format!(
            "https://api.{}/data/v1/accounts/{}/transactions?from={}&to={}",
            self.hostname(),
            account,
            from.format("%FT%TZ"),
            to.format("%FT%TZ")
        );

        let res = self
            .client
            .get(&url)
            .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
            .send()
            .await?;

        if !res.status().is_success() {
            return tl_error(res).await;
        }

        Ok(res.json::<Results<_>>().await?.results)
    }

    pub async fn pending_transactions(&self, account: &str) -> anyhow::Result<Vec<Transaction>> {
        let access_token = self.auth_provider.token_for_account(&self, account).await?;
        let url = format!(
            "https://api.{}/data/v1/accounts/{}/transactions/pending",
            self.hostname(),
            account
        );

        let res = self
            .client
            .get(&url)
            .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
            .send()
            .await?;

        if !res.status().is_success() {
            return tl_error(res).await;
        }

        Ok(res.json::<Results<_>>().await?.results)
    }
}

async fn tl_error<T>(res: reqwest::Response) -> anyhow::Result<T> {
    let status = res.status();

    if status.is_client_error() {
        let res: ErrorResponse = res.json().await?;
        return Err(anyhow!(res));
    }

    Err(anyhow!(
        "request to TrueLayer failed with status: {}",
        status
    ))
}
