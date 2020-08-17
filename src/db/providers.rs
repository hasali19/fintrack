use chrono::{DateTime, TimeZone, Utc};
use sqlx::sqlite::SqliteRow;
use sqlx::Row;

use super::Db;

pub struct Provider {
    pub id: String,
    pub display_name: String,
    pub logo_url: String,
    pub refresh_token: String,
    pub access_token: String,
    pub expires_at: DateTime<Utc>,
}

/// Gets the ids of all providers in the database.
pub async fn all_ids(db: &Db) -> anyhow::Result<Vec<String>> {
    let providers = sqlx::query("SELECT id FROM providers")
        .map(|row: SqliteRow| row.get(0))
        .fetch_all(db.pool())
        .await?;

    Ok(providers)
}

/// Inserts a new provider into the database.
///
/// Returns true if a new row was created, or false otherwise (i.e. a provider
/// with the given id already exists).
pub async fn insert(db: &Db, provider: &Provider) -> anyhow::Result<bool> {
    let sql = "
        INSERT INTO providers (id, display_name, logo_url, refresh_token, access_token, expires_at)
        VALUES (?, ?, ?, ?, ?, ?)
    ";

    let count = sqlx::query(sql)
        .bind(&provider.id)
        .bind(&provider.display_name)
        .bind(&provider.logo_url)
        .bind(&provider.refresh_token)
        .bind(&provider.access_token)
        .bind(provider.expires_at.timestamp())
        .execute(db.pool())
        .await?;

    if count > 1 {
        return Err(anyhow::anyhow!("unexpected inserted row count: {}", count));
    }

    Ok(count == 1)
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
