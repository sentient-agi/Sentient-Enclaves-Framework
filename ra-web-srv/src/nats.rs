//! NATS messaging and KV store functionality

use crate::attestation::make_attestation_docs;
use crate::config::NATSMQPersistency;
use crate::errors::{AppResult, NatsError};
use crate::state::{AppCache, AppState};
use async_nats::jetstream::context::Context as JSContext;
use async_nats::jetstream::kv::{Config as KVConfig, Operation as KVOperation, Store as KVStore};
use bytes::Bytes;
use futures::StreamExt;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

/// NATS orchestrator task: sets up client, channels, spawns core logic tasks
pub async fn nats_orchestrator(
    app_state: Arc<RwLock<AppState>>,
    app_cache: Arc<RwLock<AppCache>>,
    nats_config: NATSMQPersistency,
) -> AppResult<()> {
    let nats_url = if nats_config.nats_url.is_empty() {
        "nats://127.0.0.1:4222".to_string()
    } else {
        nats_config.nats_url
    };

    let source_bucket = if nats_config.hash_bucket_name.is_empty() {
        "fs_hashes".to_string()
    } else {
        nats_config.hash_bucket_name
    };

    let target_bucket = if nats_config.att_docs_bucket_name.is_empty() {
        "fs_att_docs".to_string()
    } else {
        nats_config.att_docs_bucket_name
    };

    // Connect to NATS with retry
    let client = loop {
        match async_nats::connect(nats_url.as_str()).await {
            Ok(conn) => break conn,
            Err(e) => {
                error!("[NATS Orchestrator] Connection failed: {}, retrying...", e);
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        }
    };
    info!("[NATS Orchestrator] Connected to NATS at {}", nats_url);

    // Check connection status
    info!("Connection status: {}", client.connection_state());

    if let Err(e) = client.flush().await {
        error!("[NATS Orchestrator] Failed to flush connection: {}", e);
        return Err(NatsError::FlushError(e.to_string()).into());
    }
    info!("Connection verified!");

    // Save NATS Client in App State
    {
        let mut app_state = app_state.write();
        app_state.nats_client = Some(client.clone());
    }

    // Get JetStream context
    let js = async_nats::jetstream::new(client);

    // Source and target buckets
    let source_kv = get_or_create_kv(&js, source_bucket.as_str()).await?;
    let target_kv = get_or_create_kv(&js, target_bucket.as_str()).await?;

    // Save target NATS KV Store in App State
    {
        let mut app_state = app_state.write();
        app_state.storage = Some(target_kv.clone());
    }

    // Channels: walker -> producer, watcher -> producer
    let (walker_tx, walker_rx) = mpsc::channel::<(String, Bytes)>(1000);
    let (watcher_tx, watcher_rx) = mpsc::channel::<(String, Bytes)>(1000);

    // Spawn pipeline of logic components
    tokio::spawn(walk_kv_entries(source_kv.clone(), walker_tx));
    tokio::spawn(watch_kv_changes(source_kv.clone(), watcher_tx));
    let app_state_clone = Arc::clone(&app_state);
    let app_cache_clone = Arc::clone(&app_cache);
    tokio::spawn(produce_kv_updates(
        walker_rx,
        watcher_rx,
        app_state_clone,
        app_cache_clone,
    ));

    Ok(())
}

/// Get or create a KV bucket
async fn get_or_create_kv(js: &JSContext, bucket_name: &str) -> AppResult<KVStore> {
    match js.get_key_value(bucket_name).await {
        Ok(kv) => {
            debug!("[NATS KV] Using existing bucket '{}'", bucket_name);
            Ok(kv)
        }
        Err(_) => {
            info!("[NATS KV] Creating bucket '{}'", bucket_name);
            js.create_key_value(KVConfig {
                bucket: bucket_name.to_string(),
                history: 10,
                ..Default::default()
            })
            .await
            .map_err(|e| {
                error!("[NATS KV] Failed to create bucket '{}': {}", bucket_name, e);
                NatsError::KvBucketError(format!("Failed to create bucket: {}", e)).into()
            })
        }
    }
}

/// Walker task: iterate through existing KV entries
async fn walk_kv_entries(store: KVStore, sender: mpsc::Sender<(String, Bytes)>) {
    info!("[NATS Walker] Walking through existing entries...");

    match store.keys().await {
        Ok(mut keys) => {
            while let Some(key_result) = keys.next().await {
                match key_result {
                    Ok(key) => match store.get(&key).await {
                        Ok(Some(bytes)) => {
                            debug!("[NATS Walker] Found key: {}", key);
                            if let Err(e) = sender.send((key.clone(), bytes)).await {
                                error!("[NATS Walker] Failed to send entry '{}': {}", key, e);
                            }
                        }
                        Ok(None) => {
                            debug!("[NATS Walker] Key '{}' not found or value not set", key);
                        }
                        Err(e) => {
                            error!("[NATS Walker] Failed to read key '{}': {}", key, e);
                        }
                    },
                    Err(e) => {
                        error!("[NATS Walker] Failed to read keys: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            error!("[NATS Walker] Unable to list keys: {}", e);
        }
    }
    info!("[NATS Walker] Finished walking entries");
}

/// Watcher task: watch for KV changes
async fn watch_kv_changes(store: KVStore, sender: mpsc::Sender<(String, Bytes)>) {
    info!("[NATS Watcher] Watching for changes...");

    match store.watch_all().await {
        Ok(mut watcher) => {
            while let Some(entry_result) = watcher.next().await {
                match entry_result {
                    Ok(entry) => {
                        let key = entry.key.clone();
                        let val = entry.value;
                        let operation = match entry.operation {
                            KVOperation::Put => "PUT",
                            KVOperation::Delete => "DELETE",
                            KVOperation::Purge => "PURGE",
                        };
                        info!(
                            "[NATS Watcher] Operation: {} | Key: {} | Value size: {} bytes",
                            operation,
                            key,
                            val.len()
                        );
                        if let Err(e) = sender.send((key.clone(), val)).await {
                            error!("[NATS Watcher] Failed to send entry '{}': {}", key, e);
                        }
                    }
                    Err(e) => {
                        error!("[NATS Watcher] Watch error: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            error!("[NATS Watcher] Failed to start watcher: {}", e);
        }
    }
}

/// Producer task: process entries and create attestation documents
async fn produce_kv_updates(
    mut walker_rx: mpsc::Receiver<(String, Bytes)>,
    mut watcher_rx: mpsc::Receiver<(String, Bytes)>,
    app_state: Arc<RwLock<AppState>>,
    app_cache: Arc<RwLock<AppCache>>,
) {
    info!("[NATS Producer] Processing walker entries...");
    while let Some((key, val)) = walker_rx.recv().await {
        debug!("[NATS Producer] Walker: Processing key '{}'", key);
        make_attestation_docs(
            key.as_str(),
            val.to_vec().as_slice(),
            Arc::clone(&app_state),
            Arc::clone(&app_cache),
        );
    }

    info!("[NATS Producer] Watching updates from watcher...");
    while let Some((key, val)) = watcher_rx.recv().await {
        debug!("[NATS Producer] Watcher: Processing key '{}'", key);
        make_attestation_docs(
            key.as_str(),
            val.to_vec().as_slice(),
            Arc::clone(&app_state),
            Arc::clone(&app_cache),
        );
    }
}
