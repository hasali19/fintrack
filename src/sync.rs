use std::collections::HashSet;
use std::sync::Arc;

use chrono::{Duration, Utc};
use true_layer::{Client as TrueLayerClient, Transaction};

use crate::{db, Db};

const FIVE_MINS: std::time::Duration = std::time::Duration::from_secs(360);

/// Clears all transactions from the database and re-fetches
/// them from the true layer api
pub async fn sync_all_transactions(db: &Db, true_layer: &TrueLayerClient) -> anyhow::Result<()> {
    db::transactions::delete_all(db).await?;

    let accounts = db::accounts::all(db).await?;
    let to = Utc::now();
    let from = to - Duration::days(365 * 6);

    for account in accounts {
        log::info!("fetching transactions for account '{}'", account.id);

        let transactions = true_layer
            .transactions(&account.id, from, to)
            .await?
            .into_iter()
            .map(|t| db::transactions::Transaction {
                id: t.transaction_id,
                account_id: account.id.to_owned(),
                timestamp: t.timestamp,
                amount: t.amount,
                currency: t.currency,
                transaction_type: Some(t.transaction_type),
                category: Some(t.transaction_category),
                description: Some(t.description),
                merchant_name: t.merchant_name,
            })
            .collect();

        db::transactions::insert_many(db, &transactions).await?;

        log::info!(
            "{} transactions inserted for account '{}'",
            transactions.len(),
            account.id
        );
    }

    Ok(())
}

pub fn start_worker(db: Db, true_layer: Arc<TrueLayerClient>) {
    tokio::task::spawn(worker(db, true_layer));
}

async fn worker(db: Db, true_layer: Arc<TrueLayerClient>) {
    loop {
        if let Err(e) = sync_transactions(&db, true_layer.as_ref()).await {
            log::error!("sync failed: {}", e);
        }
        tokio::time::delay_for(FIVE_MINS).await;
    }
}

async fn sync_transactions(db: &Db, true_layer: &TrueLayerClient) -> anyhow::Result<()> {
    let today = (Utc::now() - chrono::Duration::days(30))
        .date()
        .and_hms(0, 0, 0);

    log::info!("syncing transactions for {}", today);

    let accounts = db::accounts::all(&db).await?;
    for account in accounts {
        let saved = db::transactions::ids_after(&db, &account.id, today).await?;
        let new = true_layer
            .transactions(&account.id, today, Utc::now())
            .await?;

        if changed(&new, saved) {
            log::info!(
                "changes detected, reloading all transactions since {} for account '{}'",
                today,
                account.id
            );

            let new = new
                .into_iter()
                .map(|t| true_layer_to_db(t, &account.id))
                .collect();

            db::transactions::delete_after(&db, &account.id, today).await?;
            db::transactions::insert_many(&db, &new).await?;

            log::info!("{} transactions inserted into db", new.len());
        } else {
            log::info!(
                "no changes to transactions for account '{}' since {}, nothing to do",
                account.id,
                today
            );
        }
    }

    Ok(())
}

fn changed(new: &Vec<Transaction>, old: Vec<String>) -> bool {
    if new.len() != old.len() {
        return true;
    }

    let mut ids = HashSet::new();
    for id in old {
        ids.insert(id);
    }

    for t in new {
        if !ids.contains(&t.transaction_id) {
            return true;
        }
    }

    false
}

fn true_layer_to_db(t: Transaction, account: &str) -> db::transactions::Transaction {
    db::transactions::Transaction {
        id: t.transaction_id,
        account_id: account.to_owned(),
        timestamp: t.timestamp,
        amount: t.amount,
        currency: t.currency,
        transaction_type: Some(t.transaction_type),
        category: Some(t.transaction_category),
        description: Some(t.description),
        merchant_name: t.merchant_name,
    }
}
