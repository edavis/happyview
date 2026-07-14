use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use futures_util::stream::{self, StreamExt};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use rand::RngExt;

use crate::AppState;
use crate::db::{adapt_sql, now_rfc3339};
use crate::error::AppError;
use crate::event_log::{EventLog, Severity, log_event};
use crate::http_retry::parse_retry_after;
use crate::profile;

use super::auth::UserAuth;
use super::permissions::Permission;
use super::types::{BackfillJob, CreateBackfillBody};

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct ListReposResponse {
    repos: Vec<RepoEntry>,
    cursor: Option<String>,
}

#[derive(Deserialize)]
struct RepoEntry {
    did: String,
}

#[derive(Deserialize)]
struct ListRecordsResponse {
    records: Vec<RecordEntry>,
    cursor: Option<String>,
}

#[derive(Deserialize)]
struct RecordEntry {
    uri: String,
    cid: String,
    value: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn set_stage(state: &AppState, job_id: &str, stage: &str) {
    let sql = adapt_sql(
        "UPDATE happyview_backfill_jobs SET stage = ? WHERE id = ?",
        state.db_backend,
    );
    let _ = crate::db::query(&sql)
        .bind(stage)
        .bind(job_id)
        .execute(&state.backfill_db)
        .await;
    publish_event(
        state,
        super::types::BackfillEvent::JobStageChanged {
            job_id: job_id.to_string(),
            stage: stage.to_string(),
        },
    );
}

async fn update_job_counter(state: &AppState, job_id: &str, column: &str, value: i32) {
    let query = match column {
        "total_repos" => "UPDATE happyview_backfill_jobs SET total_repos = ? WHERE id = ?",
        "resolved_repos" => "UPDATE happyview_backfill_jobs SET resolved_repos = ? WHERE id = ?",
        "processed_repos" => "UPDATE happyview_backfill_jobs SET processed_repos = ? WHERE id = ?",
        "total_records" => "UPDATE happyview_backfill_jobs SET total_records = ? WHERE id = ?",
        other => {
            tracing::error!(
                column = other,
                "update_job_counter called with unknown column"
            );
            return;
        }
    };
    let sql = adapt_sql(query, state.db_backend);
    let _ = crate::db::query(&sql)
        .bind(value)
        .bind(job_id)
        .execute(&state.backfill_db)
        .await;
}

async fn count_repos(state: &AppState, job_id: &str) -> i32 {
    let sql = adapt_sql(
        "SELECT COUNT(*) FROM happyview_backfill_repos WHERE job_id = ?",
        state.db_backend,
    );
    crate::db::query_as::<(i32,)>(&sql)
        .bind(job_id)
        .fetch_one(&state.backfill_db)
        .await
        .map(|(c,)| c)
        .unwrap_or(0)
}

fn publish_event(state: &AppState, event: super::types::BackfillEvent) {
    let _ = state.backfill_events_tx.send(event);
}

fn random_batch_threshold(base: i32) -> i32 {
    let low = base - base / 10;
    rand::rng().random_range(low..=base)
}

struct BackfillConcurrency {
    resolution: usize,
    pds: usize,
    dids_per_pds: usize,
}

async fn load_concurrency(state: &AppState) -> BackfillConcurrency {
    let resolution = super::settings::get_setting(
        &state.db,
        "backfill_concurrent_resolution",
        state.db_backend,
    )
    .await
    .and_then(|v| v.parse().ok())
    .unwrap_or(100usize)
    .max(1);
    let pds = super::settings::get_setting(&state.db, "backfill_concurrent_pds", state.db_backend)
        .await
        .and_then(|v| v.parse().ok())
        .unwrap_or(10usize)
        .max(1);
    let dids_per_pds = super::settings::get_setting(
        &state.db,
        "backfill_concurrent_dids_per_pds",
        state.db_backend,
    )
    .await
    .and_then(|v| v.parse().ok())
    .unwrap_or(3usize)
    .max(1);
    BackfillConcurrency {
        resolution,
        pds,
        dids_per_pds,
    }
}

async fn fail_job(state: &AppState, job_id: &str, error: &str) {
    let now = now_rfc3339();
    let sql = adapt_sql(
        "UPDATE happyview_backfill_jobs SET status = 'failed', completed_at = ?, error = ? WHERE id = ?",
        state.db_backend,
    );
    let _ = crate::db::query(&sql)
        .bind(&now)
        .bind(error)
        .bind(job_id)
        .execute(&state.backfill_db)
        .await;
    publish_event(
        state,
        super::types::BackfillEvent::JobCompleted {
            job_id: job_id.to_string(),
            status: "failed".to_string(),
            error: Some(error.to_string()),
        },
    );
}

async fn should_stop(state: &AppState, job_id: &str) -> Option<&'static str> {
    let sql = adapt_sql(
        "SELECT status FROM happyview_backfill_jobs WHERE id = ?",
        state.db_backend,
    );
    let status = crate::db::query_as::<(String,)>(&sql)
        .bind(job_id)
        .fetch_optional(&state.backfill_db)
        .await
        .ok()
        .flatten()
        .map(|(s,)| s);
    match status.as_deref() {
        Some("cancelling") => Some("cancelling"),
        Some("pausing") => Some("pausing"),
        _ => None,
    }
}

async fn should_stop_worker(state: &AppState, job_id: &str) -> bool {
    should_stop(state, job_id).await.is_some()
}

async fn request_cancel(state: &AppState, job_id: &str) {
    let sql = adapt_sql(
        "UPDATE happyview_backfill_jobs SET status = 'cancelling' WHERE id = ? AND status IN ('running', 'paused')",
        state.db_backend,
    );
    let _ = crate::db::query(&sql)
        .bind(job_id)
        .execute(&state.backfill_db)
        .await;
}

async fn finalise_cancel(state: &AppState, job_id: &str) {
    let now = now_rfc3339();
    let sql = adapt_sql(
        "UPDATE happyview_backfill_jobs SET status = 'cancelled', completed_at = ?, error = 'cancelled by user' WHERE id = ?",
        state.db_backend,
    );
    let _ = crate::db::query(&sql)
        .bind(&now)
        .bind(job_id)
        .execute(&state.backfill_db)
        .await;
    publish_event(
        state,
        super::types::BackfillEvent::JobCompleted {
            job_id: job_id.to_string(),
            status: "cancelled".to_string(),
            error: None,
        },
    );
}

async fn request_pause(state: &AppState, job_id: &str) {
    let sql = adapt_sql(
        "UPDATE happyview_backfill_jobs SET status = 'pausing' WHERE id = ? AND status = 'running'",
        state.db_backend,
    );
    let _ = crate::db::query(&sql)
        .bind(job_id)
        .execute(&state.backfill_db)
        .await;
}

async fn finalise_pause(state: &AppState, job_id: &str) {
    let sql = adapt_sql(
        "UPDATE happyview_backfill_jobs SET status = 'paused' WHERE id = ?",
        state.db_backend,
    );
    let _ = crate::db::query(&sql)
        .bind(job_id)
        .execute(&state.backfill_db)
        .await;
    publish_event(
        state,
        super::types::BackfillEvent::JobCompleted {
            job_id: job_id.to_string(),
            status: "paused".to_string(),
            error: None,
        },
    );
}

async fn complete_job(
    state: &AppState,
    job_id: &str,
    processed_repos: i32,
    total_records: i32,
    error: Option<&str>,
) {
    let now = now_rfc3339();
    let sql = adapt_sql(
        "UPDATE happyview_backfill_jobs SET status = 'completed', stage = 'completed', completed_at = ?, processed_repos = ?, total_records = ?, error = ? WHERE id = ?",
        state.db_backend,
    );
    let _ = crate::db::query(&sql)
        .bind(&now)
        .bind(processed_repos)
        .bind(total_records)
        .bind(error)
        .bind(job_id)
        .execute(&state.backfill_db)
        .await;
    publish_event(
        state,
        super::types::BackfillEvent::JobCompleted {
            job_id: job_id.to_string(),
            status: "completed".to_string(),
            error: error.map(|e| e.to_string()),
        },
    );
}

// ---------------------------------------------------------------------------
// Phase 1: Discover repos via relay
// ---------------------------------------------------------------------------

async fn run_discovery_phase(
    state: &AppState,
    job_id: &str,
    collections: &[String],
    specific_did: Option<&str>,
) {
    set_stage(state, job_id, "discovering_repos").await;

    if let Some(did) = specific_did {
        let sql = adapt_sql(
            "INSERT INTO happyview_backfill_repos (job_id, did) VALUES (?, ?) ON CONFLICT DO NOTHING",
            state.db_backend,
        );
        let _ = crate::db::query(&sql)
            .bind(job_id)
            .bind(did)
            .execute(&state.backfill_db)
            .await;
        publish_event(
            state,
            super::types::BackfillEvent::RepoDiscovered {
                job_id: job_id.to_string(),
                did: did.to_string(),
            },
        );
    } else {
        stream::iter(collections.iter())
            .for_each_concurrent(5, |collection| async move {
                if should_stop_worker(state, job_id).await {
                    return;
                }
                if let Err(e) = discover_repos_from_relay(state, job_id, collection).await {
                    tracing::warn!(collection, error = %e, "failed to discover repos, skipping");
                }
            })
            .await;
    }

    let total = count_repos(state, job_id).await;
    update_job_counter(state, job_id, "total_repos", total).await;
    publish_event(
        state,
        super::types::BackfillEvent::JobCounters {
            job_id: job_id.to_string(),
            total_repos: Some(total),
            resolved_repos: None,
            processed_repos: None,
            total_records: None,
        },
    );
}

async fn discover_repos_from_relay(
    state: &AppState,
    job_id: &str,
    collection: &str,
) -> Result<(), String> {
    let base = state.config.relay_url.trim_end_matches('/');
    let mut cursor: Option<String> = None;
    let mut running_total: i32 = count_repos(state, job_id).await;

    loop {
        let mut url = format!(
            "{base}/xrpc/com.atproto.sync.listReposByCollection?collection={collection}&limit=1000"
        );
        if let Some(ref c) = cursor {
            url.push_str(&format!("&cursor={c}"));
        }

        let resp = loop {
            let r = state
                .http
                .get(&url)
                .timeout(crate::http_retry::REQUEST_TIMEOUT)
                .send()
                .await
                .map_err(|e| format!("relay request failed: {e}"))?;

            if r.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                let wait = parse_retry_after(r.headers());
                tracing::warn!(collection, wait, "rate limited by relay, sleeping");
                tokio::time::sleep(tokio::time::Duration::from_secs(wait)).await;
                continue;
            }

            break r;
        };

        if !resp.status().is_success() {
            return Err(format!("relay returned {}", resp.status()));
        }

        let body: ListReposResponse = resp
            .json()
            .await
            .map_err(|e| format!("invalid relay response: {e}"))?;

        let page_count = body.repos.len();

        if !body.repos.is_empty() {
            // SQLite has a 999 bound-parameter limit; each row uses 2 params
            let chunk_size = if state.db_backend == crate::db::DatabaseBackend::Sqlite {
                499
            } else {
                1000
            };

            for chunk in body.repos.chunks(chunk_size) {
                let base_sql = "INSERT INTO happyview_backfill_repos (job_id, did) VALUES ";
                let placeholders: Vec<String> = chunk
                    .iter()
                    .enumerate()
                    .map(|(i, _)| {
                        if state.db_backend == crate::db::DatabaseBackend::Postgres {
                            format!("(${}, ${})", i * 2 + 1, i * 2 + 2)
                        } else {
                            "(?, ?)".to_string()
                        }
                    })
                    .collect();
                let sql = format!(
                    "{base_sql}{} ON CONFLICT DO NOTHING",
                    placeholders.join(", ")
                );

                let mut query = crate::db::query(&sql);
                for repo in chunk {
                    query = query.bind(job_id).bind(&repo.did);
                }
                if let Ok(result) = query.execute(&state.backfill_db).await {
                    running_total += result.rows_affected() as i32;
                }
            }
        }

        update_job_counter(state, job_id, "total_repos", running_total).await;
        publish_event(
            state,
            super::types::BackfillEvent::JobCounters {
                job_id: job_id.to_string(),
                total_repos: Some(running_total),
                resolved_repos: None,
                processed_repos: None,
                total_records: None,
            },
        );

        if should_stop_worker(state, job_id).await {
            return Ok(());
        }

        match body.cursor {
            Some(c) if page_count > 0 => cursor = Some(c),
            _ => break,
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Phase 2: Resolve PDS endpoints for all discovered repos
// ---------------------------------------------------------------------------

async fn run_resolution_phase(state: &AppState, job_id: &str, concurrency: &BackfillConcurrency) {
    set_stage(state, job_id, "resolving_pds").await;

    // Count already-resolved repos so a resumed job reports accurate progress.
    let already_resolved: i32 = {
        let sql = adapt_sql(
            "SELECT COUNT(*) FROM happyview_backfill_repos WHERE job_id = ? AND pds_endpoint IS NOT NULL",
            state.db_backend,
        );
        crate::db::query_as::<(i32,)>(&sql)
            .bind(job_id)
            .fetch_one(&state.backfill_db)
            .await
            .map(|(c,)| c)
            .unwrap_or(0)
    };
    update_job_counter(state, job_id, "resolved_repos", already_resolved).await;

    let sql = adapt_sql(
        "SELECT did FROM happyview_backfill_repos WHERE job_id = ? AND pds_endpoint IS NULL",
        state.db_backend,
    );
    let unresolved: Vec<(String,)> = crate::db::query_as(&sql)
        .bind(job_id)
        .fetch_all(&state.backfill_db)
        .await
        .unwrap_or_default();

    let resolved = Arc::new(AtomicI32::new(already_resolved));
    let cancelled = Arc::new(AtomicBool::new(false));

    let mut attempted: i32 = 0;
    let mut next_flush = random_batch_threshold(100);
    let mut next_cancel_check = random_batch_threshold(10);

    let stream_state = state.clone();
    let stream_cancelled = Arc::clone(&cancelled);
    let mut results = stream::iter(unresolved)
        .map(move |(did,)| {
            let state = stream_state.clone();
            let cancelled = Arc::clone(&stream_cancelled);
            async move {
                if cancelled.load(Ordering::Relaxed) {
                    return None;
                }
                // Bound the entire resolution of one DID (DNS, connect, and any
                // rate-limit retry loop) so a single stuck DID can never hang
                // the resolver stream. On expiry the DID is skipped.
                let result = match tokio::time::timeout(
                    crate::http_retry::RESOLVE_DEADLINE,
                    profile::resolve_pds_endpoint(&state.http, &state.config.plc_url, &did),
                )
                .await
                {
                    Ok(result) => result,
                    Err(_) => Err(AppError::Internal(format!(
                        "PDS resolution timed out after {}s",
                        crate::http_retry::RESOLVE_DEADLINE.as_secs()
                    ))),
                };
                Some((did, result))
            }
        })
        .buffer_unordered(concurrency.resolution);

    while let Some(item) = results.next().await {
        let Some((did, result)) = item else {
            break;
        };

        match result {
            Ok(pds) => {
                let sql = adapt_sql(
                    "UPDATE happyview_backfill_repos SET pds_endpoint = ? WHERE job_id = ? AND did = ?",
                    state.db_backend,
                );
                let _ = crate::db::query(&sql)
                    .bind(&pds)
                    .bind(job_id)
                    .bind(&did)
                    .execute(&state.backfill_db)
                    .await;

                publish_event(
                    state,
                    super::types::BackfillEvent::RepoResolved {
                        job_id: job_id.to_string(),
                        did: did.clone(),
                        pds_endpoint: pds.clone(),
                    },
                );

                let count = resolved.fetch_add(1, Ordering::Relaxed) + 1;
                if count >= next_flush {
                    update_job_counter(state, job_id, "resolved_repos", count).await;
                    next_flush = count + random_batch_threshold(100);
                }
                publish_event(
                    state,
                    super::types::BackfillEvent::JobCounters {
                        job_id: job_id.to_string(),
                        total_repos: None,
                        resolved_repos: Some(count),
                        processed_repos: None,
                        total_records: None,
                    },
                );
            }
            Err(e) => {
                tracing::warn!(did, error = %e, "failed to resolve PDS endpoint, skipping DID");
            }
        }

        attempted += 1;
        if attempted >= next_cancel_check {
            if should_stop_worker(state, job_id).await {
                cancelled.store(true, Ordering::Relaxed);
                break;
            }
            next_cancel_check = attempted + random_batch_threshold(10);
        }
    }

    // Persist the final resolved count regardless of the last flush threshold.
    let final_resolved = resolved.load(Ordering::Relaxed);
    update_job_counter(state, job_id, "resolved_repos", final_resolved).await;
}

// ---------------------------------------------------------------------------
// Phase 3: Fetch records from PDS instances, grouped by PDS
// ---------------------------------------------------------------------------

async fn run_fetching_phase(
    state: &AppState,
    job_id: &str,
    collections: &[String],
    concurrency: &BackfillConcurrency,
) -> (i32, i32) {
    set_stage(state, job_id, "fetching_records").await;

    // Load pending repos grouped by PDS
    let sql = adapt_sql(
        "SELECT did, pds_endpoint FROM happyview_backfill_repos WHERE job_id = ? AND status = 'pending' AND pds_endpoint IS NOT NULL",
        state.db_backend,
    );
    let rows: Vec<(String, String)> = crate::db::query_as(&sql)
        .bind(job_id)
        .fetch_all(&state.backfill_db)
        .await
        .unwrap_or_default();

    let mut pds_to_dids: HashMap<String, Vec<String>> = HashMap::new();
    for (did, pds) in rows {
        pds_to_dids.entry(pds).or_default().push(did);
    }

    // Count already-completed repos for accurate progress
    let sql = adapt_sql(
        "SELECT COUNT(*) FROM happyview_backfill_repos WHERE job_id = ? AND status = 'completed'",
        state.db_backend,
    );
    let already_completed: i32 = crate::db::query_as::<(i32,)>(&sql)
        .bind(job_id)
        .fetch_one(&state.backfill_db)
        .await
        .map(|(c,)| c)
        .unwrap_or(0);

    // Reset processed_repos for the fetching phase
    update_job_counter(state, job_id, "processed_repos", already_completed).await;

    // Seed total_records from DB so a resumed job doesn't lose its prior count
    let existing_records: i32 = {
        let sql = adapt_sql(
            "SELECT total_records FROM happyview_backfill_jobs WHERE id = ?",
            state.db_backend,
        );
        crate::db::query_as::<(Option<i32>,)>(&sql)
            .bind(job_id)
            .fetch_one(&state.backfill_db)
            .await
            .map(|(c,)| c.unwrap_or(0))
            .unwrap_or(0)
    };

    let processed_repos = Arc::new(AtomicI32::new(already_completed));
    let total_records = Arc::new(AtomicI32::new(existing_records));
    let cancelled = Arc::new(AtomicBool::new(false));
    let next_flush = Arc::new(AtomicI32::new(
        already_completed + random_batch_threshold(10),
    ));
    let state = Arc::new(state.clone());
    let collections = Arc::new(collections.to_vec());
    let job_id_arc = Arc::new(job_id.to_string());

    let pds_entries: Vec<(String, Vec<String>)> = pds_to_dids.into_iter().collect();

    let dids_per_pds = concurrency.dids_per_pds;
    stream::iter(pds_entries)
        .for_each_concurrent(concurrency.pds, |(pds_endpoint, dids)| {
            let state = Arc::clone(&state);
            let collections = Arc::clone(&collections);
            let processed_repos = Arc::clone(&processed_repos);
            let total_records = Arc::clone(&total_records);
            let cancelled = Arc::clone(&cancelled);
            let next_flush = Arc::clone(&next_flush);
            let job_id = Arc::clone(&job_id_arc);

            async move {
                stream::iter(dids)
                    .for_each_concurrent(dids_per_pds, |did| {
                        let state = Arc::clone(&state);
                        let collections = Arc::clone(&collections);
                        let processed_repos = Arc::clone(&processed_repos);
                        let total_records = Arc::clone(&total_records);
                        let cancelled = Arc::clone(&cancelled);
                        let next_flush = Arc::clone(&next_flush);
                        let pds_endpoint = pds_endpoint.clone();
                        let job_id = Arc::clone(&job_id);

                        async move {
                            if cancelled.load(Ordering::Relaxed) {
                                return;
                            }

                            let mut did_records: i32 = 0;
                            for collection in collections.iter() {
                                if cancelled.load(Ordering::Relaxed) {
                                    break;
                                }
                                match fetch_records_from_pds(
                                    &state,
                                    &pds_endpoint,
                                    &did,
                                    collection,
                                    &cancelled,
                                )
                                .await
                                {
                                    Ok(count) => {
                                        did_records += count as i32;
                                        total_records
                                            .fetch_add(count as i32, Ordering::Relaxed);
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            did,
                                            collection,
                                            pds = %pds_endpoint,
                                            error = %e,
                                            "failed to fetch records from PDS"
                                        );
                                    }
                                }
                            }

                            // Mark DID as completed
                            let sql = adapt_sql(
                                "UPDATE happyview_backfill_repos SET status = 'completed', records_fetched = ? WHERE job_id = ? AND did = ?",
                                state.db_backend,
                            );
                            let _ = crate::db::query(&sql)
                                .bind(did_records)
                                .bind(job_id.as_str())
                                .bind(&did)
                                .execute(&state.backfill_db)
                                .await;

                            let repos = processed_repos.fetch_add(1, Ordering::Relaxed) + 1;
                            let records = total_records.load(Ordering::Relaxed);

                            let threshold = next_flush.load(Ordering::Relaxed);
                            if repos >= threshold
                                && next_flush.compare_exchange(threshold, repos + random_batch_threshold(10), Ordering::Relaxed, Ordering::Relaxed).is_ok()
                            {
                                let backend = state.db_backend;
                                let sql = adapt_sql(
                                    "UPDATE happyview_backfill_jobs SET processed_repos = ?, total_records = ? WHERE id = ?",
                                    backend,
                                );
                                let _ = crate::db::query(&sql)
                                    .bind(repos)
                                    .bind(records)
                                    .bind(job_id.as_str())
                                    .execute(&state.backfill_db)
                                    .await;

                                if should_stop_worker(&state, job_id.as_str()).await {
                                    cancelled.store(true, Ordering::Relaxed);
                                }
                            }

                            publish_event(&state, super::types::BackfillEvent::JobCounters {
                                job_id: job_id.to_string(),
                                total_repos: None,
                                resolved_repos: None,
                                processed_repos: Some(repos),
                                total_records: Some(records),
                            });
                        }
                    })
                    .await;
            }
        })
        .await;

    let final_repos = processed_repos.load(Ordering::Relaxed);
    let final_records = total_records.load(Ordering::Relaxed);

    // Persist final counts so they're accurate regardless of batch size
    let sql = adapt_sql(
        "UPDATE happyview_backfill_jobs SET processed_repos = ?, total_records = ? WHERE id = ?",
        state.db_backend,
    );
    let _ = crate::db::query(&sql)
        .bind(final_repos)
        .bind(final_records)
        .bind(job_id)
        .execute(&state.backfill_db)
        .await;

    (final_repos, final_records)
}

struct PreparedRecord {
    uri: String,
    did: String,
    collection: String,
    rkey: String,
    record_json: String,
    cid: String,
}

async fn batch_upsert_records(state: &AppState, batch: &[PreparedRecord]) {
    if batch.is_empty() {
        return;
    }

    let backend = state.db_backend;
    let now = now_rfc3339();

    // Build multi-row INSERT. 8 params per row; ON CONFLICT uses EXCLUDED.
    let placeholders: Vec<String> = (0..batch.len())
        .map(|_| "(?, ?, ?, ?, ?, ?, ?, ?)".to_string())
        .collect();
    let raw_sql = format!(
        "INSERT INTO happyview_records (uri, did, collection, rkey, record, cid, indexed_at, created_at) VALUES {} ON CONFLICT (uri) DO UPDATE SET record = EXCLUDED.record, cid = EXCLUDED.cid, indexed_at = EXCLUDED.indexed_at",
        placeholders.join(", ")
    );
    let sql = adapt_sql(&raw_sql, backend);

    let mut query = crate::db::query(&sql);
    for rec in batch {
        query = query
            .bind(&rec.uri)
            .bind(&rec.did)
            .bind(&rec.collection)
            .bind(&rec.rkey)
            .bind(&rec.record_json)
            .bind(&rec.cid)
            .bind(&now)
            .bind(&now);
    }

    if let Err(e) = query.execute(&state.backfill_db).await {
        tracing::warn!(batch_size = batch.len(), "batch record upsert failed: {e}");
    }

    // Batch sync_refs: delete old refs for all URIs, then insert new ones.
    let uris: Vec<&str> = batch.iter().map(|r| r.uri.as_str()).collect();
    let delete_placeholders: Vec<&str> = (0..uris.len()).map(|_| "?").collect();
    let delete_raw = format!(
        "DELETE FROM happyview_record_refs WHERE source_uri IN ({})",
        delete_placeholders.join(", ")
    );
    let delete_sql = adapt_sql(&delete_raw, backend);
    let mut del_query = crate::db::query(&delete_sql);
    for uri in &uris {
        del_query = del_query.bind(*uri);
    }
    let _ = del_query.execute(&state.backfill_db).await;

    // Collect all new refs and batch insert them
    let mut all_refs: Vec<(&str, String, &str)> = Vec::new();
    for rec in batch {
        let record_val: serde_json::Value =
            serde_json::from_str(&rec.record_json).unwrap_or_default();
        for target_uri in crate::record_refs::extract_at_uris(&record_val) {
            all_refs.push((&rec.uri, target_uri, &rec.collection));
        }
    }

    // Insert refs in chunks to stay within SQLite's param limit (3 params per ref)
    for chunk in all_refs.chunks(300) {
        let ref_placeholders: Vec<&str> = (0..chunk.len()).map(|_| "(?, ?, ?)").collect();
        let ref_raw = format!(
            "INSERT INTO happyview_record_refs (source_uri, target_uri, collection) VALUES {} ON CONFLICT DO NOTHING",
            ref_placeholders.join(", ")
        );
        let ref_sql = adapt_sql(&ref_raw, backend);
        let mut ref_query = crate::db::query(&ref_sql);
        for (source, target, collection) in chunk {
            ref_query = ref_query.bind(*source).bind(target).bind(*collection);
        }
        let _ = ref_query.execute(&state.backfill_db).await;
    }

    // Queue label backfill only if there are active labeler subscriptions.
    // Check once per batch instead of spawning a task per record.
    let has_subscriptions: bool = crate::db::query_as::<(i64,)>(
        "SELECT COUNT(*) FROM happyview_labeler_subscriptions WHERE status = 'active'",
    )
    .fetch_one(&state.db)
    .await
    .map(|(c,)| c > 0)
    .unwrap_or(false);

    if has_subscriptions {
        for rec in batch {
            crate::labeler::backfill_labels_for_uri(Arc::new(state.clone()), rec.uri.clone());
        }
    }
}

/// Fetch all records for a given DID and collection from a PDS via
/// `com.atproto.repo.listRecords`, paginating and handling rate limits.
async fn fetch_records_from_pds(
    state: &AppState,
    pds_endpoint: &str,
    did: &str,
    collection: &str,
    cancelled: &AtomicBool,
) -> Result<u32, String> {
    let base = pds_endpoint.trim_end_matches('/');
    let mut cursor: Option<String> = None;
    let mut count: u32 = 0;
    loop {
        if cancelled.load(Ordering::Relaxed) {
            break;
        }

        let mut url = format!(
            "{base}/xrpc/com.atproto.repo.listRecords?repo={did}&collection={collection}&limit=100"
        );
        if let Some(ref c) = cursor {
            url.push_str(&format!("&cursor={c}"));
        }

        let resp = state
            .http
            .get(&url)
            .timeout(crate::http_retry::REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(|e| format!("PDS request failed: {e}"))?;

        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let wait = parse_retry_after(resp.headers());
            tracing::warn!(did, collection, wait, "rate limited by PDS, sleeping");
            tokio::time::sleep(tokio::time::Duration::from_secs(wait)).await;
            continue;
        }

        if !resp.status().is_success() {
            return Err(format!("PDS returned {}", resp.status()));
        }

        let body: ListRecordsResponse = resp
            .json()
            .await
            .map_err(|e| format!("invalid PDS response: {e}"))?;

        let page_count = body.records.len();

        let mut batch: Vec<PreparedRecord> = Vec::with_capacity(page_count);
        for entry in &body.records {
            let rkey = entry.uri.rsplit('/').next().unwrap_or_default().to_string();
            let uri = format!("at://{did}/{collection}/{rkey}");

            // Reject records whose claimed CID doesn't match their content
            // (security review L9). The backfill source PDS is attacker-
            // controllable via the DID document, so a hostile PDS could serve a
            // record under a mismatched CID. Skip on mismatch; `Skipped`
            // (unencodable value) proceeds unchanged.
            if crate::cid_verify::verify_record_cid(&entry.cid, &entry.value)
                == crate::cid_verify::CidCheck::Mismatch
            {
                tracing::warn!(
                    collection,
                    did,
                    rkey,
                    claimed_cid = %entry.cid,
                    "record content does not match claimed CID, skipping"
                );
                continue;
            }

            let rec_to_store = match crate::lua::run_record_event_script(
                state,
                crate::lua::RecordEventPayload {
                    nsid: collection,
                    action: "create",
                    uri: &uri,
                    did,
                    rkey: &rkey,
                    record: Some(&entry.value),
                },
            )
            .await
            {
                None => continue,
                Some(v) => v,
            };

            batch.push(PreparedRecord {
                uri,
                did: did.to_string(),
                collection: collection.to_string(),
                rkey,
                record_json: serde_json::to_string(&rec_to_store).unwrap_or_default(),
                cid: entry.cid.clone(),
            });
        }

        count += batch.len() as u32;
        batch_upsert_records(state, &batch).await;

        match body.cursor {
            Some(c) if page_count > 0 => cursor = Some(c),
            _ => break,
        }
    }

    Ok(count)
}

// ---------------------------------------------------------------------------
// Background backfill worker
// ---------------------------------------------------------------------------

async fn run_backfill_job(state: AppState, job_id: String) {
    let backend = state.db_backend;

    // Load job metadata
    let sql = adapt_sql(
        "SELECT collection, did, stage FROM happyview_backfill_jobs WHERE id = ?",
        backend,
    );
    let job: Option<(Option<String>, Option<String>, String)> = crate::db::query_as(&sql)
        .bind(&job_id)
        .fetch_optional(&state.backfill_db)
        .await
        .ok()
        .flatten();

    let Some((collection, did, stage)) = job else {
        tracing::error!(job_id, "backfill job not found");
        return;
    };

    // Determine target collections
    let collections: Vec<String> = if let Some(ref col) = collection {
        let lexicon_exists: bool = state
            .lexicons
            .get(col)
            .await
            .is_some_and(|lex| lex.lexicon_type == crate::lexicon::LexiconType::Record);
        if !lexicon_exists {
            let error = format!("no record-type lexicon registered for collection '{col}'");
            fail_job(&state, &job_id, &error).await;
            return;
        }
        vec![col.clone()]
    } else {
        let sql = adapt_sql(
            "SELECT id FROM happyview_lexicons WHERE json_extract(lexicon_json, '$.defs.main.type') = 'record'",
            backend,
        );
        let rows: Vec<(String,)> = match crate::db::query_as(&sql)
            .fetch_all(&state.backfill_db)
            .await
        {
            Ok(rows) => rows,
            Err(e) => {
                let error = format!("failed to query backfill-eligible lexicons: {e}");
                fail_job(&state, &job_id, &error).await;
                return;
            }
        };
        rows.into_iter().map(|(id,)| id).collect()
    };

    if collections.is_empty() {
        complete_job(
            &state,
            &job_id,
            0,
            0,
            Some("no backfill-eligible collections"),
        )
        .await;
        return;
    }

    // Run phases, skipping those already completed
    if matches!(stage.as_str(), "pending" | "discovering_repos") {
        run_discovery_phase(&state, &job_id, &collections, did.as_deref()).await;

        match should_stop(&state, &job_id).await {
            Some("cancelling") => {
                tracing::info!(job_id, "backfill job cancelled");
                finalise_cancel(&state, &job_id).await;
                return;
            }
            Some("pausing") => {
                tracing::info!(job_id, "backfill job paused");
                finalise_pause(&state, &job_id).await;
                return;
            }
            _ => {}
        }

        let total = count_repos(&state, &job_id).await;
        if total == 0 {
            complete_job(&state, &job_id, 0, 0, None).await;
            log_event(
                &state.db,
                EventLog {
                    event_type: "backfill.completed".to_string(),
                    severity: Severity::Info,
                    actor_did: None,
                    subject: collection.clone(),
                    detail: serde_json::json!({
                        "job_id": job_id,
                        "total_repos": 0,
                        "total_records": 0,
                    }),
                },
                backend,
            )
            .await;
            return;
        }
    }

    let concurrency = load_concurrency(&state).await;

    // Resolve PDS endpoints for all discovered repos. Skipped only when a
    // resumed job has already advanced to the fetching stage; resolution is
    // idempotent (repos with a pds_endpoint already set are left untouched).
    if stage.as_str() != "fetching_records" {
        run_resolution_phase(&state, &job_id, &concurrency).await;

        match should_stop(&state, &job_id).await {
            Some("cancelling") => {
                tracing::info!(job_id, "backfill job cancelled");
                finalise_cancel(&state, &job_id).await;
                return;
            }
            Some("pausing") => {
                tracing::info!(job_id, "backfill job paused");
                finalise_pause(&state, &job_id).await;
                return;
            }
            _ => {}
        }
    }

    let (final_processed, final_records) =
        run_fetching_phase(&state, &job_id, &collections, &concurrency).await;

    match should_stop(&state, &job_id).await {
        Some("cancelling") => {
            tracing::info!(job_id, "backfill job cancelled");
            finalise_cancel(&state, &job_id).await;
            return;
        }
        Some("pausing") => {
            tracing::info!(job_id, "backfill job paused");
            finalise_pause(&state, &job_id).await;
            return;
        }
        _ => {}
    }

    complete_job(&state, &job_id, final_processed, final_records, None).await;

    log_event(
        &state.db,
        EventLog {
            event_type: "backfill.completed".to_string(),
            severity: Severity::Info,
            actor_did: None,
            subject: collection,
            detail: serde_json::json!({
                "job_id": job_id,
                "total_repos": final_processed,
                "total_records": final_records,
            }),
        },
        backend,
    )
    .await;
}

// ---------------------------------------------------------------------------
// Admin handlers
// ---------------------------------------------------------------------------

/// POST /admin/backfill — create a backfill job and spawn background work.
pub(super) async fn create_backfill(
    State(state): State<AppState>,
    admin: UserAuth,
    Json(body): Json<CreateBackfillBody>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    admin.require(Permission::BackfillCreate).await?;
    let backend = state.db_backend;

    let now = now_rfc3339();
    let job_id = Uuid::new_v4().to_string();
    let sql = adapt_sql(
        "INSERT INTO happyview_backfill_jobs (id, collection, did, status, stage, started_at, created_at) VALUES (?, ?, ?, 'running', 'pending', ?, ?) RETURNING id",
        backend,
    );
    let row: (String,) = crate::db::query_as(&sql)
        .bind(&job_id)
        .bind(&body.collection)
        .bind(&body.did)
        .bind(&now)
        .bind(&now)
        .fetch_one(&state.db)
        .await
        .map_err(|e| AppError::Internal(format!("failed to create backfill job: {e}")))?;

    let job_id = row.0.clone();

    log_event(
        &state.db,
        EventLog {
            event_type: "backfill.started".to_string(),
            severity: Severity::Info,
            actor_did: Some(admin.did.clone()),
            subject: body.collection.clone(),
            detail: serde_json::json!({
                "job_id": job_id.clone(),
            }),
        },
        backend,
    )
    .await;

    let spawn_state = state.clone();
    let spawn_job_id = job_id.clone();
    tokio::spawn(async move {
        run_backfill_job(spawn_state, spawn_job_id).await;
    });

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": job_id,
            "status": "running",
        })),
    ))
}

/// POST /admin/backfill/{id}/cancel — cancel a running backfill job.
pub(super) async fn cancel_backfill(
    State(state): State<AppState>,
    admin: UserAuth,
    Path(job_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    admin.require(Permission::BackfillCreate).await?;

    let sql = adapt_sql(
        "SELECT status FROM happyview_backfill_jobs WHERE id = ?",
        state.db_backend,
    );
    let row: Option<(String,)> = crate::db::query_as(&sql)
        .bind(&job_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| AppError::Internal(format!("failed to query backfill job: {e}")))?;

    match row {
        None => Err(AppError::NotFound("backfill job not found".into())),
        Some((ref status,)) if status == "cancelling" || status == "cancelled" => {
            Ok(Json(serde_json::json!({ "id": job_id, "status": status })))
        }
        Some((ref status,)) if status == "paused" => {
            finalise_cancel(&state, &job_id).await;
            log_event(
                &state.db,
                EventLog {
                    event_type: "backfill.cancelled".to_string(),
                    severity: Severity::Info,
                    actor_did: Some(admin.did.clone()),
                    subject: None,
                    detail: serde_json::json!({ "job_id": job_id }),
                },
                state.db_backend,
            )
            .await;
            Ok(Json(
                serde_json::json!({ "id": job_id, "status": "cancelled" }),
            ))
        }
        Some((status,)) if status != "running" => Err(AppError::BadRequest(format!(
            "job is not running (status: {status})"
        ))),
        Some(_) => {
            request_cancel(&state, &job_id).await;
            log_event(
                &state.db,
                EventLog {
                    event_type: "backfill.cancelling".to_string(),
                    severity: Severity::Info,
                    actor_did: Some(admin.did.clone()),
                    subject: None,
                    detail: serde_json::json!({ "job_id": job_id }),
                },
                state.db_backend,
            )
            .await;
            Ok(Json(
                serde_json::json!({ "id": job_id, "status": "cancelling" }),
            ))
        }
    }
}

/// POST /admin/backfill/{id}/pause — pause a running backfill job.
pub(super) async fn pause_backfill(
    State(state): State<AppState>,
    admin: UserAuth,
    Path(job_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    admin.require(Permission::BackfillCreate).await?;

    let sql = adapt_sql(
        "SELECT status FROM happyview_backfill_jobs WHERE id = ?",
        state.db_backend,
    );
    let row: Option<(String,)> = crate::db::query_as(&sql)
        .bind(&job_id)
        .fetch_optional(&state.backfill_db)
        .await
        .map_err(|e| AppError::Internal(format!("failed to query backfill job: {e}")))?;

    match row {
        None => Err(AppError::NotFound("backfill job not found".into())),
        Some((ref status,)) if status == "pausing" || status == "paused" => {
            Ok(Json(serde_json::json!({ "id": job_id, "status": status })))
        }
        Some((status,)) if status != "running" => Err(AppError::BadRequest(format!(
            "job is not running (status: {status})"
        ))),
        Some(_) => {
            request_pause(&state, &job_id).await;
            log_event(
                &state.db,
                EventLog {
                    event_type: "backfill.pausing".to_string(),
                    severity: Severity::Info,
                    actor_did: Some(admin.did.clone()),
                    subject: None,
                    detail: serde_json::json!({ "job_id": job_id }),
                },
                state.db_backend,
            )
            .await;
            Ok(Json(
                serde_json::json!({ "id": job_id, "status": "pausing" }),
            ))
        }
    }
}

/// POST /admin/backfill/{id}/resume — resume a paused backfill job.
pub(super) async fn resume_backfill(
    State(state): State<AppState>,
    admin: UserAuth,
    Path(job_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    admin.require(Permission::BackfillCreate).await?;

    let sql = adapt_sql(
        "SELECT status FROM happyview_backfill_jobs WHERE id = ?",
        state.db_backend,
    );
    let row: Option<(String,)> = crate::db::query_as(&sql)
        .bind(&job_id)
        .fetch_optional(&state.backfill_db)
        .await
        .map_err(|e| AppError::Internal(format!("failed to query backfill job: {e}")))?;

    match row {
        None => Err(AppError::NotFound("backfill job not found".into())),
        Some((status,)) if status != "paused" => Err(AppError::BadRequest(format!(
            "job is not paused (status: {status})"
        ))),
        Some(_) => {
            let sql = adapt_sql(
                "UPDATE happyview_backfill_jobs SET status = 'running' WHERE id = ?",
                state.db_backend,
            );
            let _ = crate::db::query(&sql)
                .bind(&job_id)
                .execute(&state.backfill_db)
                .await;

            let spawn_state = state.clone();
            let spawn_job_id = job_id.clone();
            tokio::spawn(async move {
                run_backfill_job(spawn_state, spawn_job_id).await;
            });

            log_event(
                &state.db,
                EventLog {
                    event_type: "backfill.resumed".to_string(),
                    severity: Severity::Info,
                    actor_did: Some(admin.did.clone()),
                    subject: None,
                    detail: serde_json::json!({ "job_id": job_id }),
                },
                state.db_backend,
            )
            .await;
            Ok(Json(
                serde_json::json!({ "id": job_id, "status": "running" }),
            ))
        }
    }
}

/// GET /admin/backfill/status — list all backfill jobs.
pub(super) async fn backfill_status(
    State(state): State<AppState>,
    auth: UserAuth,
) -> Result<Json<Vec<BackfillJob>>, AppError> {
    auth.require(Permission::BackfillRead).await?;
    let backend = state.db_backend;

    let sql = adapt_sql(
        "SELECT id, collection, did, status, stage, total_repos, resolved_repos, processed_repos, total_records, error, started_at, completed_at, created_at FROM happyview_backfill_jobs ORDER BY created_at DESC",
        backend,
    );
    #[allow(clippy::type_complexity)]
    let rows: Vec<(
        String,
        Option<String>,
        Option<String>,
        String,
        String,
        Option<i32>,
        Option<i32>,
        Option<i32>,
        Option<i32>,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
    )> = crate::db::query_as(&sql)
        .fetch_all(&state.backfill_db)
        .await
        .map_err(|e| AppError::Internal(format!("failed to list backfill jobs: {e}")))?;

    let jobs: Vec<BackfillJob> = rows
        .into_iter()
        .map(
            |(
                id,
                collection,
                did,
                status,
                stage,
                total_repos,
                resolved_repos,
                processed_repos,
                total_records,
                error,
                started_at,
                completed_at,
                created_at,
            )| {
                BackfillJob {
                    id,
                    collection,
                    did,
                    status,
                    stage,
                    total_repos,
                    resolved_repos,
                    processed_repos,
                    total_records,
                    error,
                    started_at,
                    completed_at,
                    created_at,
                }
            },
        )
        .collect();

    Ok(Json(jobs))
}

// ---------------------------------------------------------------------------
// SSE events endpoint
// ---------------------------------------------------------------------------

pub(super) async fn backfill_events(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
    auth: UserAuth,
) -> Result<
    axum::response::sse::Sse<
        impl futures_util::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>,
    >,
    AppError,
> {
    auth.require(Permission::BackfillRead).await?;

    let mut rx = state.backfill_events_tx.subscribe();

    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let event_job_id = match &event {
                        super::types::BackfillEvent::RepoDiscovered { job_id, .. }
                        | super::types::BackfillEvent::RepoResolved { job_id, .. }
                        | super::types::BackfillEvent::RepoFetched { job_id, .. }
                        | super::types::BackfillEvent::JobCounters { job_id, .. }
                        | super::types::BackfillEvent::JobStageChanged { job_id, .. }
                        | super::types::BackfillEvent::JobCompleted { job_id, .. } => job_id,
                    };
                    if *event_job_id != job_id {
                        continue;
                    }
                    if let Ok(json) = serde_json::to_string(&event) {
                        yield Ok(axum::response::sse::Event::default().event("event").data(json));
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(job_id, skipped = n, "SSE client lagged behind");
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    };

    Ok(axum::response::sse::Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default()))
}

// ---------------------------------------------------------------------------
// REST detail endpoints
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub(super) struct ReposQuery {
    phase: Option<String>,
    cursor: Option<String>,
    limit: Option<i32>,
}

pub(super) async fn backfill_repos(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
    auth: UserAuth,
    axum::extract::Query(query): axum::extract::Query<ReposQuery>,
) -> Result<Json<super::types::BackfillReposResponse>, AppError> {
    auth.require(Permission::BackfillRead).await?;

    let limit = query.limit.unwrap_or(50).min(100);
    let phase_filter = match query.phase.as_deref() {
        Some("resolved") => " AND pds_endpoint IS NOT NULL",
        Some("fetched") => " AND status = 'completed'",
        _ => "",
    };
    let cursor_filter = if query.cursor.is_some() {
        " AND did > ?"
    } else {
        ""
    };

    let sql_str = format!(
        "SELECT did, pds_endpoint, status, records_fetched FROM happyview_backfill_repos WHERE job_id = ?{phase_filter}{cursor_filter} ORDER BY did ASC LIMIT ?",
    );
    let sql = adapt_sql(&sql_str, state.db_backend);

    let mut q = crate::db::query_as::<(String, Option<String>, String, i32)>(&sql).bind(&job_id);
    if let Some(ref cursor) = query.cursor {
        q = q.bind(cursor);
    }
    q = q.bind(limit + 1);

    let rows: Vec<(String, Option<String>, String, i32)> = q
        .fetch_all(&state.backfill_db)
        .await
        .map_err(|e| AppError::Internal(format!("failed to query backfill repos: {e}")))?;

    let has_more = rows.len() > limit as usize;
    let repos: Vec<super::types::BackfillRepoEntry> = rows
        .into_iter()
        .take(limit as usize)
        .map(
            |(did, pds_endpoint, status, records_fetched)| super::types::BackfillRepoEntry {
                did,
                pds_endpoint,
                status,
                records_fetched,
            },
        )
        .collect();

    let cursor = if has_more {
        repos.last().map(|r| r.did.clone())
    } else {
        None
    };

    Ok(Json(super::types::BackfillReposResponse { repos, cursor }))
}

pub(super) async fn backfill_pds_summary(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
    auth: UserAuth,
) -> Result<Json<super::types::PdsSummaryResponse>, AppError> {
    auth.require(Permission::BackfillRead).await?;

    let sql = adapt_sql(
        "SELECT pds_endpoint, COUNT(*) as total_repos, SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as completed_repos, SUM(records_fetched) as total_records FROM happyview_backfill_repos WHERE job_id = ? AND pds_endpoint IS NOT NULL GROUP BY pds_endpoint ORDER BY COUNT(*) DESC",
        state.db_backend,
    );

    let rows: Vec<(String, i32, i32, i64)> = crate::db::query_as(&sql)
        .bind(&job_id)
        .fetch_all(&state.backfill_db)
        .await
        .map_err(|e| AppError::Internal(format!("failed to query PDS summary: {e}")))?;

    let pds_endpoints: Vec<super::types::PdsSummaryEntry> = rows
        .into_iter()
        .map(
            |(pds_endpoint, total_repos, completed_repos, total_records)| {
                super::types::PdsSummaryEntry {
                    pds_endpoint,
                    total_repos,
                    completed_repos,
                    total_records: total_records as i32,
                }
            },
        )
        .collect();

    Ok(Json(super::types::PdsSummaryResponse { pds_endpoints }))
}

// ---------------------------------------------------------------------------
// Flush endpoints
// ---------------------------------------------------------------------------

pub(super) async fn flush_backfill_details(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
    auth: UserAuth,
) -> Result<StatusCode, AppError> {
    auth.require(Permission::BackfillCreate).await?;

    let sql = adapt_sql(
        "DELETE FROM happyview_backfill_repos WHERE job_id = ?",
        state.db_backend,
    );
    let _ = crate::db::query(&sql)
        .bind(&job_id)
        .execute(&state.backfill_db)
        .await;

    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn flush_all_backfill_details(
    State(state): State<AppState>,
    auth: UserAuth,
) -> Result<StatusCode, AppError> {
    auth.require(Permission::BackfillCreate).await?;

    let sql = adapt_sql(
        "DELETE FROM happyview_backfill_repos WHERE job_id IN (SELECT id FROM happyview_backfill_jobs WHERE status IN ('completed', 'cancelled', 'failed'))",
        state.db_backend,
    );
    let _ = crate::db::query(&sql).execute(&state.backfill_db).await;

    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Retention cleanup
// ---------------------------------------------------------------------------

pub async fn run_backfill_retention_cleanup(state: &AppState) {
    use super::settings::get_setting;

    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(86400));

    loop {
        interval.tick().await;

        let retention_days: i64 = get_setting(
            &state.backfill_db,
            "backfill_retention_days",
            state.db_backend,
        )
        .await
        .and_then(|v| v.parse().ok())
        .unwrap_or(28);

        if retention_days == 0 {
            continue;
        }

        let cutoff = chrono::Utc::now() - chrono::Duration::days(retention_days);
        let cutoff_str = cutoff.to_rfc3339();

        let sql = adapt_sql(
            "DELETE FROM happyview_backfill_repos WHERE job_id IN (SELECT id FROM happyview_backfill_jobs WHERE completed_at IS NOT NULL AND completed_at < ?)",
            state.db_backend,
        );
        match crate::db::query(&sql)
            .bind(&cutoff_str)
            .execute(&state.backfill_db)
            .await
        {
            Ok(result) => {
                let deleted = result.rows_affected();
                if deleted > 0 {
                    tracing::info!(
                        deleted,
                        retention_days,
                        "cleaned up old backfill detail rows"
                    );
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "backfill retention cleanup failed");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Startup resumption
// ---------------------------------------------------------------------------

/// Resume any backfill jobs that were running when the server last stopped.
/// Jobs stuck in `cancelling` are finalised immediately.
pub async fn resume_backfill_jobs(state: &AppState) {
    let sql = adapt_sql(
        "SELECT id, status FROM happyview_backfill_jobs WHERE status IN ('running', 'cancelling', 'pausing')",
        state.db_backend,
    );
    let rows: Vec<(String, String)> = crate::db::query_as(&sql)
        .fetch_all(&state.backfill_db)
        .await
        .unwrap_or_default();

    for (job_id, status) in rows {
        match status.as_str() {
            "cancelling" => {
                tracing::info!(
                    job_id,
                    "finalising cancelled backfill job from previous run"
                );
                finalise_cancel(state, &job_id).await;
            }
            "pausing" => {
                tracing::info!(job_id, "finalising paused backfill job from previous run");
                finalise_pause(state, &job_id).await;
            }
            _ => {
                tracing::info!(job_id, "resuming interrupted backfill job");
                let spawn_state = state.clone();
                tokio::spawn(async move {
                    run_backfill_job(spawn_state, job_id).await;
                });
            }
        }
    }
}
