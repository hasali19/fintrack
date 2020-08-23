use std::collections::HashSet;
use std::sync::Arc;

use chrono::{Duration, Utc};
use true_layer::{Client as TrueLayerClient, Transaction};

use crate::{db, Db};

const FIVE_MINS: std::time::Duration = std::time::Duration::from_secs(300);

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
    let today = Utc::now().date().and_hms(0, 0, 0);

    for account in db::accounts::all(&db).await? {
        if db::transactions::has_any(db, &account.id).await? {
            log::info!(
                "syncing transactions since {} for account '{}'",
                today,
                account.id
            );

            let saved = db::transactions::ids_after(&db, &account.id, today).await?;
            let new = true_layer
                .transactions(&account.id, today, Utc::now())
                .await?;

            if changed(&new, saved) {
                log::info!(
                    "changes detected for account '{}', refreshing transactions",
                    account.id
                );

                let new = new
                    .into_iter()
                    .map(|t| true_layer_to_db(t, &account.id))
                    .collect::<Vec<_>>();

                db::transactions::delete_after(&db, &account.id, today).await?;
                db::transactions::insert_many(&db, &new).await?;

                log::info!("{} transactions inserted into db", new.len());
            } else {
                log::info!(
                    "no changes detected for account '{}', nothing to do",
                    account.id,
                );
            }
        } else {
            log::info!(
                "first sync for account '{}', fetching all transactions",
                account.id
            );

            let to = Utc::now();
            let from = to - Duration::days(365 * 6);

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
                .collect::<Vec<_>>();

            db::transactions::insert_many(db, &transactions).await?;

            log::info!("{} transactions inserted into db", transactions.len());
        }
    }

    Ok(())
}

fn changed(new: &[Transaction], old: Vec<String>) -> bool {
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
