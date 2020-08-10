use std::ops::Deref;
use std::sync::Arc;

use crate::config::Config;
use crate::db::Db;
use crate::true_layer::{AuthProvider as TrueLayerAuthProvider, Client as TrueLayerClient};

#[derive(Clone)]
pub struct State(Arc<StateInner>);

impl State {
    pub fn new(db: Db, auth_provider: impl TrueLayerAuthProvider + Send + Sync + 'static) -> State {
        State(Arc::new(StateInner {
            db,
            config: Config::from_env(),
            true_layer: TrueLayerClient::new(auth_provider),
        }))
    }
}

impl Deref for State {
    type Target = StateInner;
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

pub struct StateInner {
    db: Db,
    config: Config,
    true_layer: TrueLayerClient,
}

impl StateInner {
    pub fn db(&self) -> &Db {
        &self.db
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn true_layer(&self) -> &TrueLayerClient {
        &self.true_layer
    }
}
