use serde::Serialize;
use tide::http::headers;
use tide::{Response, StatusCode};

use crate::Request;

pub trait RequestExt {
    fn render_template<T: Serialize>(&self, name: &str, data: &T) -> tide::Result;
}

impl RequestExt for Request {
    fn render_template<T: Serialize>(&self, name: &str, data: &T) -> tide::Result {
        let hb = self.state().handlebars();
        let res = Response::builder(StatusCode::Ok)
            .header(headers::CONTENT_TYPE, "text/html")
            .body(hb.render(name, data)?)
            .build();

        Ok(res)
    }
}
