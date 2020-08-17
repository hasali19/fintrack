use actix_files::{Files, NamedFile};
use actix_web::{
    middleware::Logger,
    web::{self, Data},
    App, HttpServer, Responder,
};

use fintrack::cron;
use fintrack::utils::AuthProvider;
use fintrack::{db, services, Config, Db};
use true_layer::Client as TrueLayerClient;

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    femme::start();

    let config = Config::from_env();
    let db = Db::connect("sqlite://fintrack.db").await?;
    let true_layer = Data::new(TrueLayerClient::new(AuthProvider::new(db.clone())));

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
                .service(services::connect("/connect"))
                .service(services::api("/api"))
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

async fn spa_index() -> actix_web::Result<impl Responder> {
    Ok(NamedFile::open("client/build/index.html")?)
}

async fn fetch_new_providers(db: &Db, true_layer: &TrueLayerClient) -> anyhow::Result<()> {
    let providers = true_layer.supported_providers().await?;
    let known = db::providers::all_ids(db).await?;

    let mut count = 0i32;

    for provider in providers.iter().filter(|p| !known.contains(&p.provider_id)) {
        log::info!("adding new provider '{}'", provider.provider_id);

        let id = &provider.provider_id;
        let name = &provider.display_name;
        let logo = &provider.logo_url;

        db::providers::insert(db, id, name, logo).await?;

        count += 1;
    }

    if count == 0 {
        log::info!("no new providers found");
    } else {
        log::info!("{} new providers were added", count);
    }

    Ok(())
}

async fn fetch_accounts(db: &Db, true_layer: &TrueLayerClient) -> anyhow::Result<()> {
    let providers = db::providers::connected_ids(db).await?;

    let mut total = 0;

    for provider in providers {
        let accounts = true_layer.accounts(&provider).await?;
        for account in accounts {
            let id = &account.account_id;
            let name = &account.display_name;
            let created = db::accounts::insert(db, id, &provider, name).await?;

            if created {
                total += 1;
                log::info!("new account '{}' added to db", account.display_name);
            } else {
                log::info!("account '{}' already exists", account.display_name);
            }
        }
    }

    log::info!("{} new accounts added to the db", total);

    Ok(())
}
