use chrono::{DateTime, TimeZone, Utc};
use sqlx::sqlite::SqliteRow;
use sqlx::Row;

use super::Db;

/// Gets the ids of all providers in the database.
pub async fn all_ids(db: &Db) -> anyhow::Result<Vec<String>> {
    let providers = sqlx::query("SELECT id FROM providers")
        .map(|row: SqliteRow| row.get(0))
        .fetch_all(db.pool())
        .await?;

    Ok(providers)
}

/// Gets the id of all connected providers in the database.
///
/// A connected provider is one that has a non-null refresh_token.
pub async fn connected_ids(db: &Db) -> anyhow::Result<Vec<String>> {
    let providers = sqlx::query("SELECT id FROM providers WHERE refresh_token IS NOT NULL")
        .map(|row: SqliteRow| row.get(0))
        .fetch_all(db.pool())
        .await?;

    Ok(providers)
}

/// Inserts a new provider into the database.
pub async fn insert(db: &Db, id: &str, display_name: &str, logo_url: &str) -> anyhow::Result<()> {
    sqlx::query("INSERT INTO providers (id, display_name, logo_url) VALUES (?, ?, ?)")
        .bind(id)
        .bind(display_name)
        .bind(logo_url)
        .execute(db.pool())
        .await?;

    Ok(())
}

/// Gets the saved credentials (access_token, expires_at, refresh_token) for
/// a particular provider.
pub async fn credentials(db: &Db, id: &str) -> anyhow::Result<(String, DateTime<Utc>, String)> {
    let sql = "
        SELECT access_token, expires_at, refresh_token
        FROM providers
        WHERE id = ?
    ";

    let cred = sqlx::query(sql)
        .bind(id)
        .map(|row: SqliteRow| {
            let expires_at = Utc.timestamp(row.get(1), 0);
            (row.get(0), expires_at, row.get(2))
        })
        .fetch_one(db.pool())
        .await?;

    Ok(cred)
}

/// Updates the saved credentials for a particular provider.
pub async fn update_credentials(
    db: &Db,
    id: &str,
    access_token: &str,
    expires_at: DateTime<Utc>,
    refresh_token: &str,
) -> anyhow::Result<()> {
    let sql = "
        UPDATE providers
        SET access_token = ?, expires_at = ?, refresh_token = ?
        WHERE id = ?
    ";

    sqlx::query(sql)
        .bind(access_token)
        .bind(expires_at.timestamp())
        .bind(refresh_token)
        .bind(id)
        .execute(db.pool())
        .await?;

    Ok(())
}
