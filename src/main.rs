use tide::http::cookies::SameSite;
use tide::sessions::SessionMiddleware;
use tide::utils::After;
use tide::{log, Body, Redirect, Response};

use async_ctrlc::CtrlC;
use async_sqlx_session::SqliteSessionStore;
use async_std::prelude::FutureExt;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqliteRow;
use sqlx::Row;

use fintrack::prelude::*;
use fintrack::{cron, true_layer};
use fintrack::{Db, Request, State};

struct AuthProvider(Db);

#[async_std::main]
async fn main() -> tide::Result<()> {
    dotenv::dotenv().ok();
    log::start();

    let db = Db::connect("sqlite://fintrack.db").await?;
    let state = State::new(db.clone(), AuthProvider(db.clone()));

    cron::new("update_providers", "0 0 0 * * * *")
        .with_state(state.clone())
        .spawn_with_task(|state| async move {
            let res = fetch_new_providers(state.db(), state.true_layer()).await;
            if let Err(e) = res {
                log::error!("failed to fetch truelayer providers: {}", e.to_string());
            }
        });

    let ctrlc = CtrlC::new()?;
    let mut app = tide::with_state(state.clone());

    app.with({
        let key = &state.config().secret_key;
        let store = SqliteSessionStore::from_client(db.pool().clone());

        store.migrate().await?;

        SessionMiddleware::new(store, key)
            .with_same_site_policy(SameSite::Lax)
            .with_cookie_name("fintrack.sid")
    });

    app.with(After(error_handler));

    app.at("/").get(index);
    app.at("/connect").get(connect);
    app.at("/connect/callback").get(callback);

    let mut api = app.at("/api");

    api.at("/providers").get(get_connected_providers);
    api.at("/accounts").get(get_accounts);

    let address = &state.config().http_address;
    let port = state.config().http_port;

    app.listen(format!("{}:{}", address, port))
        .race(async {
            ctrlc.await;
            Ok(())
        })
        .await?;

    db.close().await;

    Ok(())
}

async fn fetch_new_providers(db: &Db, true_layer: &true_layer::Client) -> anyhow::Result<()> {
    let providers = true_layer.supported_providers().await?;
    let known: Vec<String> = sqlx::query("SELECT id FROM providers")
        .map(|row: SqliteRow| row.get(0))
        .fetch_all(db.pool())
        .await?;

    let mut count = 0;

    for provider in providers.iter().filter(|p| !known.contains(&p.provider_id)) {
        log::info!("adding new provider '{}'", provider.provider_id);
        sqlx::query("INSERT INTO providers (id, display_name, logo_url) VALUES (?, ?, ?)")
            .bind(&provider.provider_id)
            .bind(&provider.display_name)
            .bind(&provider.logo_url)
            .execute(db.pool())
            .await?;
        count += 1;
    }

    if count == 0 {
        log::info!("no new providers found");
    } else {
        log::info!("{} new providers were added", count);
    }

    Ok(())
}

async fn error_handler(mut res: Response) -> tide::Result {
    if let Some(err) = res.take_error() {
        res.set_body(err.to_string());
    }
    Ok(res)
}

async fn index(_: Request) -> tide::Result {
    Ok("FinTrack".into())
}

async fn connect(req: Request) -> tide::Result {
    let true_layer = req.state().true_layer();
    let location = true_layer.auth_link(&req.url_for("connect/callback")?);
    Ok(Redirect::new(location).into())
}

#[derive(Serialize, Deserialize)]
struct CallbackQuerySuccess {
    code: String,
    scope: String,
}

#[derive(Deserialize)]
struct CallbackQueryError {
    error: String,
}

async fn callback(req: Request) -> tide::Result {
    if let Ok(CallbackQueryError { error }) = req.query() {
        return Ok(error.into());
    }

    let params: CallbackQuerySuccess = req.query()?;
    let true_layer = req.state().true_layer();

    let token_res = true_layer
        .exchange_code(&params.code, &req.url_for("connect/callback")?)
        .await?;

    save_credentials(req.state().db(), true_layer, token_res).await?;

    Ok(Redirect::new(req.url_for("")?).into())
}

async fn save_credentials(
    db: &Db,
    true_layer: &true_layer::Client,
    token_res: true_layer::TokenResponse,
) -> anyhow::Result<String> {
    let metadata = true_layer.token_metadata(&token_res.access_token).await?;
    let expires_at = Utc::now() + Duration::seconds(token_res.expires_in);

    let sql = "
        UPDATE providers
        SET access_token = ?, expires_at = ?, refresh_token = ?
        WHERE id = ?
    ";

    sqlx::query(sql)
        .bind(&token_res.access_token)
        .bind(expires_at.timestamp())
        .bind(&token_res.refresh_token)
        .bind(&metadata.provider.provider_id)
        .execute(db.pool())
        .await?;

    Ok(token_res.access_token)
}

#[derive(Serialize)]
struct Provider {
    id: String,
    name: String,
    logo: String,
}

async fn get_connected_providers(req: Request) -> tide::Result {
    let sql = "
        SELECT id, display_name, logo_url
        FROM providers
        WHERE refresh_token IS NOT NULL
    ";

    let providers = sqlx::query(sql)
        .map(|row: SqliteRow| Provider {
            id: row.get(0),
            name: row.get(1),
            logo: row.get(2),
        })
        .fetch_all(req.state().db().pool())
        .await?;

    Ok(Body::from_json(&providers)?.into())
}

#[derive(Deserialize)]
struct GetAccountsQuery {
    provider: String,
}

async fn get_accounts(req: Request) -> tide::Result {
    let query: GetAccountsQuery = req.query()?;
    let true_layer = req.state().true_layer();
    let accounts = true_layer.accounts(&query.provider).await?;
    Ok(Body::from_json(&accounts)?.into())
}

#[async_trait::async_trait]
impl true_layer::AuthProvider for AuthProvider {
    async fn access_token(
        &self,
        provider: &str,
        true_layer: &true_layer::Client,
    ) -> anyhow::Result<String> {
        let sql = "
            SELECT access_token, expires_at, refresh_token
            FROM providers
            WHERE id = ?
        ";

        let (access_token, expires_at, refresh_token): (String, i64, String) = sqlx::query(sql)
            .bind(provider)
            .map(|row: SqliteRow| (row.get(0), row.get(1), row.get(2)))
            .fetch_one(self.0.pool())
            .await?;

        if expires_at > Utc::now().timestamp() {
            return Ok(access_token);
        }

        let token_res = true_layer.renew_token(&refresh_token).await?;

        Ok(save_credentials(&self.0, true_layer, token_res).await?)
    }
}
