use std::path::Path;

use actix_files::NamedFile;
use actix_web::{
    middleware::Logger,
    web::{self, Data},
    App, HttpRequest, HttpServer, Responder,
};

use env_logger::Env;
use fintrack::utils::AuthProvider;
use fintrack::{services, Config, Db};
use true_layer::Client as TrueLayerClient;

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    env_logger::from_env(Env::default().default_filter_or("info")).init();

    let config = Config::from_env();
    let db = Db::connect(&config.db_url).await?;
    let true_layer = Data::new(TrueLayerClient::new(AuthProvider::new(db.clone())));

    fintrack::migrations::run(&db).await?;
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
                .default_service(web::get().to(spa_fallback))
        }
    })
    .bind(format!("{}:{}", address, port))?
    .run()
    .await?;

    db.close().await;

    Ok(())
}

async fn spa_fallback(req: HttpRequest) -> actix_web::Result<impl Responder> {
    let path = Path::new("client/build").join(req.path().trim_start_matches('/'));
    if path.is_file() {
        Ok(NamedFile::open(path)?)
    } else {
        Ok(NamedFile::open("client/build/index.html")?)
    }
}
