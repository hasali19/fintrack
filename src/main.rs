mod config;
mod db;
mod ext;
mod state;
mod true_layer;

use tide::http::cookies::SameSite;
use tide::sessions::SessionMiddleware;
use tide::utils::After;
use tide::{Redirect, Response};

use async_ctrlc::CtrlC;
use async_sqlx_session::SqliteSessionStore;
use async_std::prelude::FutureExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
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
    let state = State::new();

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
    app.at("/callback").get(callback);

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

async fn index(req: Request) -> tide::Result {
    let session = req.session();
    let true_layer = req.state().true_layer();

    match session.get("authenticated") {
        Some(true) => {}
        _ => return Ok(Redirect::new(true_layer.auth_link()).into()),
    };

    let data = json!({
        "title": "Home",
        "message": "Authenticated",
    });

    req.render_template("index", &data)
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

async fn callback(mut req: Request) -> tide::Result {
    if let Ok(CallbackQueryError { error }) = req.query() {
        return Ok(error.into());
    }

    let params: CallbackQuerySuccess = req.query()?;
    let true_layer = req.state().true_layer();
    let _ = true_layer.exchange_code(&params.code).await?;

    req.session_mut().insert("authenticated", true)?;

    Ok(Redirect::new("http://localhost:8000").into())
}
