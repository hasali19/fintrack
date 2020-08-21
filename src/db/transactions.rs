use chrono::{DateTime, Utc};
use sqlx::sqlite::SqliteRow;
use sqlx::Row;

use super::Db;

pub struct Transaction {
    pub id: String,
    pub account_id: String,
    pub timestamp: DateTime<Utc>,
    pub amount: f64,
    pub currency: String,
    pub transaction_type: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
    pub merchant_name: Option<String>,
}

pub async fn ids_after(
    db: &Db,
    account: &str,
    timestamp: DateTime<Utc>,
) -> anyhow::Result<Vec<String>> {
    let sql = "
        SELECT id
        FROM transactions
        WHERE account_id = ? AND timestamp >= ?
    ";

    let transactions = sqlx::query(sql)
        .bind(account)
        .bind(timestamp.timestamp())
        .map(|row: SqliteRow| row.get(0))
        .fetch_all(db.pool())
        .await?;

    Ok(transactions)
}

pub async fn insert_many(db: &Db, transactions: &Vec<Transaction>) -> anyhow::Result<()> {
    for chunk in transactions.chunks(100) {
        let mut sql = "
            INSERT INTO transactions (
                id,
                account_id,
                timestamp,
                amount,
                currency,
                type,
                category,
                description,
                merchant_name
            ) VALUES
        "
        .to_owned();

        for i in 0..chunk.len() {
            sql.push_str(" (?, ?, ?, ?, ?, ?, ?, ?, ?)");
            if i != chunk.len() - 1 {
                sql.push_str(", ");
            }
        }

        chunk
            .iter()
            .fold(sqlx::query(&sql), |query, t| {
                query
                    .bind(&t.id)
                    .bind(&t.account_id)
                    .bind(t.timestamp.timestamp())
                    .bind(t.amount)
                    .bind(&t.currency)
                    .bind(&t.transaction_type)
                    .bind(&t.category)
                    .bind(&t.description)
                    .bind(&t.merchant_name)
            })
            .execute(db.pool())
            .await?;
    }

    Ok(())
}

pub async fn delete_all(db: &Db) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM transactions")
        .execute(db.pool())
        .await?;

    log::info!("all transactions deleted from db");

    Ok(())
}

pub async fn delete_after(db: &Db, account: &str, timestamp: DateTime<Utc>) -> anyhow::Result<()> {
    let sql = "
        DELETE FROM transactions
        WHERE account_id = ? AND timestamp >= ?
    ";

    let rows = sqlx::query(sql)
        .bind(account)
        .bind(timestamp.date().and_hms(0, 0, 0).timestamp())
        .execute(db.pool())
        .await?;

    log::info!("{} transactions deleted from db", rows);

    Ok(())
}
