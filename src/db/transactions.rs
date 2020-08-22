use chrono::{DateTime, Utc};
use sqlx::postgres::PgRow;
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
    let sql = "SELECT id FROM transactions WHERE account_id = $1 AND timestamp >= $2";

    let transactions = sqlx::query(sql)
        .bind(account)
        .bind(timestamp)
        .map(|row: PgRow| row.get(0))
        .fetch_all(db.pool())
        .await?;

    Ok(transactions)
}

pub async fn insert_many(db: &Db, transactions: &[Transaction]) -> anyhow::Result<()> {
    for chunk in transactions.chunks(100) {
        let mut sql = "
            INSERT INTO transactions (
                id, account_id, timestamp, amount, currency,
                type, category, description, merchant_name
            ) VALUES
        "
        .to_owned();

        // FIXME: Well this is horrible
        for i in 0..chunk.len() {
            sql += " (";
            for j in 0..9 {
                sql += "$";
                itoa::fmt(&mut sql, i * 9 + j + 1)?;
                if j < 8 {
                    sql += ", ";
                }
            }
            sql += ")";
            if i != chunk.len() - 1 {
                sql += ", ";
            }
        }

        chunk
            .iter()
            .fold(sqlx::query(&sql), |query, t| {
                query
                    .bind(&t.id)
                    .bind(&t.account_id)
                    .bind(t.timestamp)
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
        WHERE account_id = $1 AND timestamp >= $2
    ";

    let rows = sqlx::query(sql)
        .bind(account)
        .bind(timestamp.date().and_hms(0, 0, 0))
        .execute(db.pool())
        .await?;

    log::info!("{} transactions deleted from db", rows);

    Ok(())
}
