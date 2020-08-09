mod config;
mod db;
mod ext;
mod state;
mod true_layer;

use tide::http::cookies::SameSite;
use tide::sessions::SessionMiddleware;
use tide::utils::After;
use tide::{Redirect, Response, StatusCode};

use async_ctrlc::CtrlC;
use async_sqlx_session::SqliteSessionStore;
use async_std::prelude::FutureExt;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqliteRow;
use sqlx::Row;

use crate::db::Db;
use crate::ext::*;
use crate::state::State;

type Request = tide::Request<State>;

#[async_std::main]
async fn main() -> tide::Result<()> {
    dotenv::dotenv().ok();
    tide::log::start();

    let address = "127.0.0.1";
    let port = 8000;

    let db = Db::connect("sqlite://fintrack.db").await?;
    let state = State::new(db.clone());

    fetch_new_providers(&db, state.true_layer()).await?;

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

    api.at("/session").get(get_session);
    api.at("/providers").get(get_connected_providers);

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

    for provider in providers.iter().filter(|p| !known.contains(&p.provider_id)) {
        tide::log::info!("adding new provider '{}'", provider.provider_id);
        sqlx::query("INSERT INTO providers (id, display_name, logo_url) VALUES (?, ?, ?)")
            .bind(&provider.provider_id)
            .bind(&provider.display_name)
            .bind(&provider.logo_url)
            .execute(db.pool())
            .await?;
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

    let token = true_layer
        .exchange_code(&params.code, &req.url_for("connect/callback")?)
        .await?;

    let metadata = true_layer.token_metadata(&token.access_token).await?;
    let expires_at = Utc::now() + Duration::seconds(token.expires_in);

    let sql = "
        UPDATE providers
        SET access_token = ?, expires_at = ?, refresh_token = ?
        WHERE id = ?
    ";

    sqlx::query(sql)
        .bind(&token.access_token)
        .bind(expires_at.timestamp())
        .bind(&token.refresh_token)
        .bind(&metadata.provider.provider_id)
        .execute(req.state().db().pool())
        .await?;

    Ok(Redirect::new(req.url_for("")?).into())
}

#[derive(Serialize)]
struct SessionState {
    authenticated: bool,
}

async fn get_session(req: Request) -> tide::Result {
    let session = req.session();
    let state = SessionState {
        authenticated: session.get("authenticated").unwrap_or(false),
    };

    Ok(Response::builder(StatusCode::Ok)
        .content_type("application/json")
        .body(serde_json::to_vec(&state)?)
        .build())
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

    Ok(Response::builder(StatusCode::Ok)
        .content_type("application/json")
        .body(serde_json::to_vec(&providers)?)
        .build())
}
