use actix_web::{
    dev::HttpServiceFactory,
    error::ErrorBadRequest,
    web::{self, Data, Query},
    HttpResponse, Responder,
};

use serde::Deserialize;

pub fn service(path: &str) -> impl HttpServiceFactory {
    web::scope(path).route("/api/accounts", web::get().to(get_accounts))
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
