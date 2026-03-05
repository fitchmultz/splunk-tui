//! Purpose: Provide CLI entrypoints for transaction lifecycle commands.
//! Responsibilities: Execute begin/status/commit/rollback/archive flows via the shared transaction manager.
//! Non-scope: Does not implement transaction persistence/rollback internals (handled in client crate).
//! Invariants/Assumptions: At most one pending transaction file exists per profile scope.

use crate::args::TransactionCommand;
use crate::commands::get_transaction_manager;
use anyhow::{Context, Result};
use splunk_client::transaction::Transaction;

pub async fn run(
    config: splunk_config::Config,
    command: TransactionCommand,
    no_cache: bool,
) -> Result<()> {
    let manager = get_transaction_manager()?;

    match command {
        TransactionCommand::Begin => {
            if manager.load_pending().await?.is_some() {
                anyhow::bail!("A transaction is already in progress. Commit or rollback first.");
            }
            let transaction = Transaction::new();
            manager.save_pending(&transaction).await?;
            println!("Started new transaction: {}", transaction.id);
        }
        TransactionCommand::Commit { dry_run } => {
            let transaction = manager
                .load_pending()
                .await?
                .context("No transaction in progress. Run 'splunk-cli transaction begin' first.")?;

            let client = crate::commands::build_client_from_config(&config, Some(no_cache))?;

            if dry_run {
                println!("Validating transaction {}...", transaction.id);
                manager.validate(&client, &transaction).await?;
                println!("Transaction is valid. Staged operations:");
                for (i, op) in transaction.operations.iter().enumerate() {
                    println!("  {}. {:?}", i + 1, op);
                }
            } else {
                println!("Committing transaction {}...", transaction.id);
                match manager.commit(&client, &transaction).await {
                    Ok(_) => {
                        manager.archive(&transaction, "committed").await?;
                        manager.clear_pending().await?;
                        println!("Transaction committed successfully.");
                    }
                    Err(e) => {
                        manager.archive(&transaction, "failed").await?;
                        // We don't clear pending on failure to allow manual recovery or retry?
                        // Actually, the manager performs automatic rollback.
                        // If rollback succeeds, we should probably clear pending.
                        // For now, let's follow the plan and report failure.
                        anyhow::bail!("Transaction failed: {}", e);
                    }
                }
            }
        }
        TransactionCommand::Rollback => {
            let transaction = manager
                .load_pending()
                .await?
                .context("No transaction in progress.")?;

            manager.archive(&transaction, "rolled_back").await?;
            manager.clear_pending().await?;
            println!(
                "Transaction {} rolled back (staged operations cleared).",
                transaction.id
            );
        }
        TransactionCommand::Status => match manager.load_pending().await? {
            Some(transaction) => {
                println!("Transaction in progress: {}", transaction.id);
                println!("Created at: {}", transaction.created_at);
                println!("Staged operations: {}", transaction.operations.len());
                for (i, op) in transaction.operations.iter().enumerate() {
                    println!("  {}. {:?}", i + 1, op);
                }
                if !transaction.savepoints.is_empty() {
                    println!("Savepoints:");
                    for (name, pos) in &transaction.savepoints {
                        println!("  - {} (at position {})", name, pos);
                    }
                }
            }
            None => println!("No transaction in progress."),
        },
        TransactionCommand::Savepoint { name } => {
            let mut transaction = manager
                .load_pending()
                .await?
                .context("No transaction in progress.")?;

            transaction.set_savepoint(name.clone());
            manager.save_pending(&transaction).await?;
            println!(
                "Created savepoint '{}' at position {}",
                name,
                transaction.operations.len()
            );
        }
    }

    Ok(())
}
