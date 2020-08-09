use std::ops::Deref;
use std::sync::Arc;

use crate::config::Config;
use crate::true_layer::Client as TrueLayerClient;

#[derive(Clone)]
pub struct State(Arc<StateInner>);

impl State {
    pub fn new() -> State {
        State(Arc::new(StateInner {
            config: Config::from_env(),
            true_layer: TrueLayerClient::new(),
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
    config: Config,
    true_layer: TrueLayerClient,
}

impl StateInner {
    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn true_layer(&self) -> &TrueLayerClient {
        &self.true_layer
    }
}
