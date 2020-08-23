pub mod accounts;
pub mod providers;
pub mod transactions;

use sqlx::PgPool;

#[derive(Clone)]
pub struct Db(PgPool);

impl Db {
    pub async fn connect(url: &str) -> sqlx::Result<Db> {
        Ok(Db(PgPool::connect(url).await?))
    }

    pub fn pool(&self) -> &PgPool {
        &self.0
    }

    pub async fn close(self) {
        self.0.close().await
    }
}
