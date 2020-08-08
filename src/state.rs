use std::ops::Deref;
use std::sync::Arc;

use handlebars::Handlebars;

#[derive(Clone)]
pub struct State(Arc<StateInner>);

impl State {
    pub fn new() -> State {
        let mut handlebars = Handlebars::new();

        handlebars
            .register_templates_directory(".html", "static/templates")
            .unwrap();

        State(Arc::new(StateInner { handlebars }))
    }
}

impl Deref for State {
    type Target = StateInner;
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

pub struct StateInner {
    handlebars: Handlebars<'static>,
}

impl StateInner {
    pub fn handlebars(&self) -> &Handlebars {
        &self.handlebars
    }
}
