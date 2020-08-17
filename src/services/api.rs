use actix_web::{
    dev::HttpServiceFactory,
    error::ErrorInternalServerError,
    web::{self, Data, Path},
    HttpResponse, Responder,
};

use serde_json::json;

use crate::{db, Db};

pub fn service(path: &str) -> impl HttpServiceFactory {
    web::scope(path)
        .route("/accounts", web::get().to(get_accounts))
        .route("/accounts/{id}/balance", web::get().to(get_account_balance))
        .default_service(web::route().to(|| {
            HttpResponse::NotFound().json(&json!({
                "error": "not_found"
            }))
        }))
}

async fn get_accounts(db: Db) -> actix_web::Result<impl Responder> {
    let accounts = db::accounts::all(&db)
        .await
        .map_err(|_| ErrorInternalServerError("failed to get accounts from db"))?;

    Ok(HttpResponse::Ok().json(accounts))
}

async fn get_account_balance(
    path: Path<(String,)>,
    true_layer: Data<true_layer::Client>,
) -> actix_web::Result<impl Responder> {
    let (account_id,) = path.into_inner();
    let balance = true_layer
        .account_balance(&account_id)
        .await
        .map_err(|_| ErrorInternalServerError("failed to get account balance"))?;

    Ok(HttpResponse::Ok().json(balance))
}
