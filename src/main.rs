use actix_files::{Files, NamedFile};
use actix_web::{
    error::{ErrorBadRequest, ErrorInternalServerError},
    guard,
    http::header,
    middleware::Logger,
    web::{self, Data, Query},
    App, HttpRequest, HttpResponse, HttpServer, Responder,
};

use async_trait::async_trait;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqliteRow;
use sqlx::Row;

use fintrack::cron;
use fintrack::{Config, Db};

struct AuthProvider(Db);

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    femme::start();

    let config = Config::from_env();
    let db = Db::connect("sqlite://fintrack.db").await?;
    let true_layer = Data::new(true_layer::Client::new(AuthProvider(db.clone())));

    fetch_new_providers(&db, true_layer.as_ref()).await?;
    fetch_accounts(&db, true_layer.as_ref()).await?;

    cron::new("update_providers", "0 0 0 * * * *")
        .with_state((db.clone(), true_layer.clone()))
        .spawn_with_task(|(db, true_layer)| async move {
            let res = fetch_new_providers(&db, true_layer.as_ref()).await;
            if let Err(e) = res {
                log::error!("failed to fetch truelayer providers: {}", e.to_string());
            }
        });

    let address = &config.http_address;
    let port = config.http_port;

    HttpServer::new({
        let db = db.clone();
        move || {
            App::new()
                .wrap(Logger::default())
                .app_data(db.clone())
                .app_data(true_layer.clone())
                .route("/connect", web::get().to(connect))
                .service(
                    web::resource("/connect/callback")
                        .name("connect_callback")
                        .guard(guard::Get())
                        .to(callback),
                )
                .route("/api/providers", web::get().to(get_connected_providers))
                .route("/api/accounts", web::get().to(get_accounts))
                .service(Files::new("/static", "client/build/static"))
                .default_service(web::get().to(spa_index))
        }
    })
    .bind(format!("{}:{}", address, port))?
    .run()
    .await?;

    db.close().await;

    Ok(())
}

async fn fetch_new_providers(db: &Db, true_layer: &true_layer::Client) -> anyhow::Result<()> {
    let providers = true_layer.supported_providers().await?;
    let known = sqlx::query("SELECT id FROM providers")
        .map(|row: SqliteRow| row.get(0))
        .fetch_all(db.pool())
        .await?;

    let mut count = 0i32;

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

async fn fetch_accounts(db: &Db, true_layer: &true_layer::Client) -> anyhow::Result<()> {
    let providers: Vec<String> =
        sqlx::query("SELECT id FROM providers WHERE refresh_token IS NOT NULL")
            .map(|row: SqliteRow| row.get(0))
            .fetch_all(db.pool())
            .await?;

    let mut total = 0;

    for provider in providers {
        let accounts = true_layer.accounts(&provider).await?;
        for account in accounts {
            let sql = "
                INSERT INTO accounts (id, provider_id, display_name)
                VALUES (?, ?, ?)
                ON CONFLICT DO NOTHING
            ";

            let count = sqlx::query(sql)
                .bind(&account.account_id)
                .bind(&provider)
                .bind(&account.display_name)
                .execute(db.pool())
                .await?;

            match count {
                0 => log::info!("account '{}' already exists", account.display_name),
                1 => log::info!("new account '{}' added to db", account.display_name),
                _ => panic!("unexpected number of updated rows when inserting account"),
            }

            total += count;
        }
    }

    log::info!("{} new accounts added to the db", total);

    Ok(())
}

async fn spa_index() -> actix_web::Result<impl Responder> {
    Ok(NamedFile::open("client/build/index.html")?)
}

async fn connect(
    req: HttpRequest,
    true_layer: Data<true_layer::Client>,
) -> actix_web::Result<impl Responder> {
    let callback = req.url_for_static("connect_callback")?;
    let location = true_layer.auth_link(callback.as_str());
    Ok(HttpResponse::TemporaryRedirect()
        .set_header(header::LOCATION, location)
        .finish())
}

#[derive(Serialize, Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    scope: Option<String>,
    error: Option<String>,
}

async fn callback(
    req: HttpRequest,
    Query(query): Query<CallbackQuery>,
    true_layer: Data<true_layer::Client>,
    db: Db,
) -> actix_web::Result<impl Responder> {
    if let Some(error) = query.error {
        return Err(ErrorBadRequest(error));
    }

    let (code, _) = match (query.code, query.scope) {
        (Some(code), Some(scope)) => (code, scope),
        _ => {
            return Err(ErrorBadRequest(
                "'code' and 'scope' query parameters must be provided",
            ))
        }
    };

    let token_res = true_layer
        .exchange_code(&code, req.url_for_static("connect_callback")?.as_str())
        .await
        .map_err(|_| ErrorInternalServerError("failed to exchange code for auth token"))?;

    save_credentials(&db, true_layer.as_ref(), token_res)
        .await
        .map_err(|_| ErrorInternalServerError("failed to save credentials"))?;

    let index = format!(
        "{}://{}",
        req.connection_info().scheme(),
        req.connection_info().host()
    );

    Ok(HttpResponse::TemporaryRedirect()
        .set_header(header::LOCATION, index)
        .finish())
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

async fn get_connected_providers(db: Db) -> actix_web::Result<impl Responder> {
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
        .fetch_all(db.pool())
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(&providers))
}

#[derive(Deserialize)]
struct GetAccountsQuery {
    provider: String,
}

async fn get_accounts(
    Query(query): Query<GetAccountsQuery>,
    true_layer: Data<true_layer::Client>,
) -> actix_web::Result<impl Responder> {
    let accounts = true_layer
        .accounts(&query.provider)
        .await
        .map_err(ErrorBadRequest)?;

    Ok(HttpResponse::Ok().json(accounts))
}

#[async_trait]
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
