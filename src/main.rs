use actix_files::{Files, NamedFile};
use actix_web::{
    middleware::Logger,
    web::{self, Data},
    App, HttpServer, Responder,
};

use fintrack::utils::AuthProvider;
use fintrack::{services, Config, Db};
use true_layer::Client as TrueLayerClient;

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    femme::start();

    let config = Config::from_env();
    let db = Db::connect(&config.db_url()).await?;
    let true_layer = Data::new(TrueLayerClient::new(AuthProvider::new(db.clone())));

    fintrack::sync::start_worker(db.clone(), true_layer.clone().into_inner());

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
