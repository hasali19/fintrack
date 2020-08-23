use chrono::{DateTime, TimeZone, Utc};
use sqlx::postgres::PgRow;
use sqlx::{Done, Row};

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
        .try_map(|row: PgRow| Ok(row.get(0)))
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
        VALUES ($1, $2, $3, $4, $5, $6)
    ";

    let count = sqlx::query(sql)
        .bind(&provider.id)
        .bind(&provider.display_name)
        .bind(&provider.logo_url)
        .bind(&provider.refresh_token)
        .bind(&provider.access_token)
        .bind(provider.expires_at.timestamp())
        .execute(db.pool())
        .await?
        .rows_affected();

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
        WHERE id = $1
    ";

    let cred = sqlx::query(sql)
        .bind(id)
        .try_map(|row: PgRow| Ok((row.get(0), Utc.from_utc_datetime(&row.get(1)), row.get(2))))
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
        SET access_token = $1, expires_at = $2, refresh_token = $3
        WHERE id = $4
    ";

    sqlx::query(sql)
        .bind(access_token)
        .bind(expires_at)
        .bind(refresh_token)
        .bind(id)
        .execute(db.pool())
        .await?;

    Ok(())
}
