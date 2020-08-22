pub mod accounts;
pub mod providers;
pub mod transactions;

use sqlx::{Executor, PgPool};

#[derive(Clone)]
pub struct Db(PgPool);

impl Db {
    pub async fn connect(url: &str) -> sqlx::Result<Db> {
        let pool = PgPool::new(url).await?;

        pool.acquire()
            .await?
            .execute(include_str!("schema.sql"))
            .await?;

        Ok(Db(pool))
    }

    pub fn pool(&self) -> &PgPool {
        &self.0
    }

    pub async fn close(self) {
        self.0.close().await
    }
}
