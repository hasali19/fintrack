use async_trait::async_trait;
use chrono::{Duration, Utc};

use crate::{db, Db};

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

pub struct AuthProvider(Db);

impl AuthProvider {
    pub fn new(db: Db) -> AuthProvider {
        AuthProvider(db)
    }
}

#[async_trait]
impl true_layer::AuthProvider for AuthProvider {
    async fn access_token(
        &self,
        provider: &str,
        true_layer: &true_layer::Client,
    ) -> anyhow::Result<String> {
        let (access_token, expires_at, refresh_token) =
            db::providers::credentials(&self.0, provider).await?;

        if expires_at > Utc::now() {
            return Ok(access_token);
        }

        let token_res = true_layer.renew_token(&refresh_token).await?;

        Ok(save_credentials(&self.0, true_layer, token_res).await?)
    }
}
