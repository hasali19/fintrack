use anyhow::anyhow;
use rust_embed::RustEmbed;
use sqlx::{postgres::PgRow, Cursor, Executor, Row};

use crate::Db;

#[derive(RustEmbed)]
#[folder = "migrations"]
struct Migration;

pub async fn run(db: &Db) -> anyhow::Result<()> {
    let mut db_version = get_current_version(&db).await?;

    for file in Migration::iter() {
        let version: i32 = file.split('_').next().unwrap().parse()?;
        if version <= db_version {
            continue;
        }

        let bytes = Migration::get(&file).ok_or_else(|| anyhow!("failed to read '{}'", file))?;
        let sql = String::from_utf8(bytes.to_vec())?;

        log::info!("running migration '{}'", file);

        let mut transaction = db.pool().begin().await?;
        let update_query = format!("UPDATE _migration_version SET version = {}", version);

        transaction.execute(sql.as_str()).await?;
        transaction.execute(update_query.as_str()).await?;

        transaction.commit().await?;

        db_version = version;
    }

    log::info!("database is up to date");

    Ok(())
}

async fn get_current_version(db: &Db) -> anyhow::Result<i32> {
    let sql = "
        CREATE TABLE IF NOT EXISTS _migration_version (
            version INTEGER PRIMARY KEY
        );

        INSERT INTO _migration_version
        SELECT 0
        WHERE NOT EXISTS (SELECT 1 FROM _migration_version);

        SELECT version FROM _migration_version;
    ";

    let mut pool = db.pool();
    let mut cursor = pool.fetch(sql);
    let row: Option<PgRow> = cursor.next().await?;
    let version: Option<i32> = row.unwrap().get(0);

    Ok(version.unwrap_or(0))
}
