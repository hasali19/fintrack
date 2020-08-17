use actix_web::{
    dev::HttpServiceFactory, error::ErrorInternalServerError, web, HttpResponse, Responder,
};

use crate::{db, Db};

pub fn service(path: &str) -> impl HttpServiceFactory {
    web::scope(path).route("/accounts", web::get().to(get_accounts))
}

async fn get_accounts(db: Db) -> actix_web::Result<impl Responder> {
    let accounts = db::accounts::all(&db)
        .await
        .map_err(|_| ErrorInternalServerError("failed to get accounts from db"))?;

    Ok(HttpResponse::Ok().json(accounts))
}
