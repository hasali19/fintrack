use serde::Serialize;
use sqlx::sqlite::SqliteRow;
use sqlx::Row;

use super::Db;

#[derive(Debug, Serialize)]
pub struct Account {
    pub id: String,
    pub provider_id: String,
    pub display_name: String,
}

/// Gets all accounts from the database.
pub async fn all(db: &Db) -> anyhow::Result<Vec<Account>> {
    let accounts = sqlx::query("SELECT id, provider_id, display_name FROM accounts")
        .map(|row: SqliteRow| Account {
            id: row.get(0),
            provider_id: row.get(1),
            display_name: row.get(2),
        })
        .fetch_all(db.pool())
        .await?;

    Ok(accounts)
}

/// Inserts a new account into the database.
///
/// Returns true if a new row was created, or false otherwise (i.e. an account
/// with the given id already exists).
pub async fn insert(db: &Db, id: &str, provider: &str, display_name: &str) -> anyhow::Result<bool> {
    let sql = "
        INSERT INTO accounts (id, provider_id, display_name)
        VALUES (?, ?, ?)
        ON CONFLICT DO NOTHING
    ";

    let count = sqlx::query(sql)
        .bind(id)
        .bind(provider)
        .bind(display_name)
        .execute(db.pool())
        .await?;

    if count > 1 {
        return Err(anyhow::anyhow!("unexpected inserted row count: {}", count));
    }

    Ok(count == 1)
}
