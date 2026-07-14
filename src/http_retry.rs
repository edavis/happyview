use std::time::Duration;

/// Per-request timeout for outbound atproto network fetches (relay, PLC/DID
/// resolution, PDS reads). Bounds a single HTTP attempt so a host that connects
/// but never responds fails instead of stalling a backfill job forever.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Overall deadline for resolving a single DID's PDS endpoint. Unlike
/// [`REQUEST_TIMEOUT`], this bounds the *entire* resolution of one DID —
/// including DNS, connection setup, and any rate-limit retry/backoff loop — so
/// a single stuck DID can never hang the resolver stream. On expiry the DID is
/// treated as a resolution failure and skipped, and the backfill moves on.
pub const RESOLVE_DEADLINE: Duration = Duration::from_secs(30);

/// Parse rate-limit sleep duration from response headers.
/// Checks `RateLimit-Reset` (Unix timestamp, used by XRPC servers) first,
/// then `retry-after` (seconds), defaulting to 5s.
pub fn parse_retry_after(headers: &reqwest::header::HeaderMap) -> u64 {
    if let Some(reset) = headers
        .get("ratelimit-reset")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<i64>().ok())
    {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let wait = (reset - now).max(1) as u64;
        return wait.min(120);
    }

    headers
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(5)
        .min(120)
}
