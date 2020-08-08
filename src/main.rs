mod ext;
mod state;

use serde_json::json;

use crate::ext::*;
use crate::state::State;

type Request = tide::Request<State>;

#[async_std::main]
async fn main() -> std::io::Result<()> {
    tide::log::start();

    let address = "127.0.0.1";
    let port = 8000;

    let mut app = tide::with_state(State::new());

    app.at("/").get(index);
    app.listen(format!("{}:{}", address, port)).await?;

    Ok(())
}

async fn index(req: Request) -> tide::Result {
    let data = json!({
        "title": "Home",
        "message": "Hello, world!",
    });

    req.render_template("index", &data)
}
