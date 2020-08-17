use actix_web::{
    dev::HttpServiceFactory,
    error::{ErrorBadRequest, ErrorInternalServerError},
    guard,
    http::header,
    web::{self, Data, Query},
    HttpRequest, HttpResponse, Responder,
};

use serde::{Deserialize, Serialize};

use crate::{utils, Db};

pub fn service(path: &str) -> impl HttpServiceFactory {
    web::scope(path).route("/", web::get().to(connect)).service(
        web::resource("/callback")
            .name("connect_callback")
            .guard(guard::Get())
            .to(callback),
    )
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

    utils::save_credentials(&db, true_layer.as_ref(), token_res)
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
