use std::ops::Deref;
use std::sync::Arc;

use handlebars::Handlebars;

use crate::config::Config;
use crate::true_layer::Client as TrueLayerClient;

#[derive(Clone)]
pub struct State(Arc<StateInner>);

impl State {
    pub fn new() -> State {
        let mut handlebars = Handlebars::new();

        handlebars
            .register_templates_directory(".html", "static/templates")
            .unwrap();

        State(Arc::new(StateInner {
            handlebars,
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
    handlebars: Handlebars<'static>,
    true_layer: TrueLayerClient,
}

impl StateInner {
    pub fn handlebars(&self) -> &Handlebars {
        &self.handlebars
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn true_layer(&self) -> &TrueLayerClient {
        &self.true_layer
    }
}
