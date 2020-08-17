use super::Db;

/// Insert a new account into the database.
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
