use chrono::{DateTime, TimeZone, Utc};
use serde::Serialize;
use sqlx::postgres::PgRow;
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
        .map(|row: PgRow| Account {
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
        VALUES ($1, $2, $3)
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

/// Gets the saved credentials (access_token, expires_at, refresh_token) for
/// a particular account.
pub async fn credentials(db: &Db, id: &str) -> anyhow::Result<(String, DateTime<Utc>, String)> {
    let sql = "
        SELECT access_token, expires_at, refresh_token
        FROM accounts AS a JOIN providers AS p
        ON a.provider_id = p.id
        WHERE a.id = $1
    ";

    let cred = sqlx::query(sql)
        .bind(id)
        .map(|row: PgRow| (row.get(0), Utc.from_utc_datetime(&row.get(1)), row.get(2)))
        .fetch_one(db.pool())
        .await?;

    Ok(cred)
}
