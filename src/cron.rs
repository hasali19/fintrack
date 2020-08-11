use std::{future::Future, str::FromStr};

use async_std::task;
use chrono::Utc;
use cron::Schedule;
use tide::log;

pub struct Builder {
    name: String,
    schedule: Schedule,
}

pub fn new(name: &str, cron: &str) -> Builder {
    Builder {
        name: name.to_owned(),
        schedule: Schedule::from_str(cron).unwrap(),
    }
}

impl Builder {
    pub fn with_state<S: Clone>(self, state: S) -> StatefulBuilder<S> {
        StatefulBuilder {
            name: self.name,
            schedule: self.schedule,
            state,
        }
    }

    pub fn spawn_with_task<R, F>(self, task: F)
    where
        R: Future<Output = ()> + Send + 'static,
        F: Fn(()) -> R + Send + 'static,
    {
        self.with_state(()).spawn_with_task(task);
    }

    pub async fn run_with_task<R, F>(self, task: F)
    where
        R: Future<Output = ()> + Send + 'static,
        F: Fn(()) -> R + Send + 'static,
    {
        self.with_state(()).run_with_task(task).await;
    }
}

pub struct StatefulBuilder<S: Clone> {
    name: String,
    schedule: Schedule,
    state: S,
}

impl<S: Clone + Send + 'static> StatefulBuilder<S> {
    pub fn spawn_with_task<R, F>(self, task: F)
    where
        R: Future<Output = ()> + Send + 'static,
        F: Fn(S) -> R + Send + 'static,
    {
        task::spawn(self.run_with_task(task));
    }

    async fn run_with_task<R, F>(self, task: F)
    where
        R: Future<Output = ()> + Send + 'static,
        F: Fn(S) -> R + Send + 'static,
    {
        for next in self.schedule.upcoming(Utc) {
            log::info!("next run of '{}' is scheduled for {}", self.name, next);
            let dur = next - Utc::now();
            let dur = dur.to_std().unwrap();
            task::sleep(dur).await;

            log::info!("running task '{}'", self.name);
            task::spawn((task)(self.state.clone()));
        }
    }
}
