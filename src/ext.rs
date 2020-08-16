use actix_web::{dev::Payload, FromRequest, HttpRequest};
use futures::future::Ready;

use crate::Db;

impl FromRequest for Db {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        futures::future::ok(req.app_data::<Db>().unwrap().clone())
    }
}
