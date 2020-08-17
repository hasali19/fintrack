use async_trait::async_trait;
use chrono::{Duration, Utc};

use crate::db::{self, Db};

pub async fn save_credentials(
    db: &Db,
    true_layer: &true_layer::Client,
    token_res: true_layer::TokenResponse,
) -> anyhow::Result<String> {
    let metadata = true_layer.token_metadata(&token_res.access_token).await?;
    let expires_at = Utc::now() + Duration::seconds(token_res.expires_in);

    let id = &metadata.provider.provider_id;
    let access_token = &token_res.access_token;
    let refresh_token = &token_res.refresh_token;

    db::providers::update_credentials(db, id, access_token, expires_at, refresh_token).await?;

    Ok(token_res.access_token)
}

pub async fn fetch_provider_accounts(
    db: &Db,
    true_layer: &true_layer::Client,
    provider: &str,
) -> anyhow::Result<()> {
    let accounts = true_layer.accounts(&provider).await?;

    for account in accounts {
        let id = &account.account_id;
        let name = &account.display_name;
        let created = db::accounts::insert(db, id, &provider, name).await?;
        if created {
            log::info!("new account '{}' added to db", account.display_name);
        } else {
            log::info!("account '{}' already exists", account.display_name);
        }
    }

    Ok(())
}

pub struct AuthProvider(Db);

impl AuthProvider {
    pub fn new(db: Db) -> AuthProvider {
        AuthProvider(db)
    }
}

#[async_trait]
impl true_layer::AuthProvider for AuthProvider {
    async fn token_for_provider(
        &self,
        true_layer: &true_layer::Client,
        provider_id: &str,
    ) -> anyhow::Result<String> {
        let (access_token, expires_at, refresh_token) =
            db::providers::credentials(&self.0, provider_id).await?;

        if expires_at > Utc::now() {
            return Ok(access_token);
        }

        let token_res = true_layer.renew_token(&refresh_token).await?;
        let access_token = save_credentials(&self.0, true_layer, token_res).await?;

        Ok(access_token)
    }

    async fn token_for_account(
        &self,
        true_layer: &true_layer::Client,
        account_id: &str,
    ) -> anyhow::Result<String> {
        let (access_token, expires_at, refresh_token) =
            db::accounts::credentials(&self.0, account_id).await?;

        if expires_at > Utc::now() {
            return Ok(access_token);
        }

        let token_res = true_layer.renew_token(&refresh_token).await?;
        let access_token = save_credentials(&self.0, true_layer, token_res).await?;

        Ok(access_token)
    }
}
