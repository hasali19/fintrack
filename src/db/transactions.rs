use chrono::{DateTime, TimeZone, Utc};
use rust_decimal::Decimal;
use sqlx::postgres::PgRow;
use sqlx::{Done, Row};

use super::Db;

#[derive(Debug, serde::Serialize)]
pub struct Transaction {
    pub id: String,
    pub account_id: String,
    pub timestamp: DateTime<Utc>,
    pub amount: Decimal,
    pub currency: String,
    pub transaction_type: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
    pub merchant_name: Option<String>,
}

/// Returns true if there are any recorded transactions
/// for the specified account.
pub async fn has_any(db: &Db, account: &str) -> anyhow::Result<bool> {
    let res: Option<i32> = sqlx::query("SELECT 1 FROM transactions WHERE account_id = $1")
        .bind(account)
        .try_map(|row: PgRow| Ok(row.get(0)))
        .fetch_optional(db.pool())
        .await?;

    Ok(res.is_some())
}

/// Returns all transactions for the given account.
pub async fn all(db: &Db, account: &str) -> anyhow::Result<Vec<Transaction>> {
    let query = "
        SELECT id, account_id, timestamp, amount, currency,
               type, category, description, merchant_name
        FROM transactions
        WHERE account_id = $1
    ";

    let transactions = sqlx::query(query)
        .bind(account)
        .try_map(|row: PgRow| {
            Ok(Transaction {
                id: row.get(0),
                account_id: row.get(1),
                timestamp: Utc.from_utc_datetime(&row.get(2)),
                amount: row.get(3),
                currency: row.get(4),
                transaction_type: row.get(5),
                category: row.get(6),
                description: row.get(7),
                merchant_name: row.get(8),
            })
        })
        .fetch_all(db.pool())
        .await?;

    Ok(transactions)
}

/// Returns a list of transaction ids for all transactions
/// made since the specified timestamp.
pub async fn ids_after(
    db: &Db,
    account: &str,
    timestamp: DateTime<Utc>,
) -> anyhow::Result<Vec<String>> {
    let sql = "
        SELECT id FROM transactions
        WHERE account_id = $1 AND timestamp >= $2
    ";

    let transactions = sqlx::query(sql)
        .bind(account)
        .bind(timestamp)
        .try_map(|row: PgRow| Ok(row.get(0)))
        .fetch_all(db.pool())
        .await?;

    Ok(transactions)
}

/// Inserts multiple transaction records into the database.
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

/// Deletes ***all*** transactions from the database.
pub async fn delete_all(db: &Db) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM transactions")
        .execute(db.pool())
        .await?;

    log::info!("all transactions deleted from db");

    Ok(())
}

/// Deletes all transactions for the specified account that were
/// made since the given timestamp.
pub async fn delete_after(db: &Db, account: &str, timestamp: DateTime<Utc>) -> anyhow::Result<()> {
    let sql = "
        DELETE FROM transactions
        WHERE account_id = $1 AND timestamp >= $2
    ";

    let count = sqlx::query(sql)
        .bind(account)
        .bind(timestamp.date().and_hms(0, 0, 0))
        .execute(db.pool())
        .await?
        .rows_affected();

    log::info!("{} transactions deleted from db", count);

    Ok(())
}
