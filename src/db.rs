pub mod accounts;
pub mod providers;
pub mod transactions;

use sqlx::SqlitePool;

#[derive(Clone)]
pub struct Db(SqlitePool);

impl Db {
    pub async fn connect(url: &str) -> sqlx::Result<Db> {
        let pool = SqlitePool::new(url).await?;

        sqlx::query(include_str!("schema.sql"))
            .execute(&pool)
            .await?;

        Ok(Db(pool))
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.0
    }

    pub async fn close(self) {
        self.0.close().await
    }
}
