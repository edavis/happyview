use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use p256::ecdsa::{Signature, VerifyingKey, signature::Verifier};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use crate::error::AppError;

/// How far in the past a DPoP proof's `iat` may be and still be accepted.
const DPOP_IAT_PAST_TOLERANCE_SECS: u64 = 300;
/// How far in the future a DPoP proof's `iat` may be — small clock-skew slack.
/// Previously the past tolerance (300s) was also applied to the future via
/// `abs_diff`, accepting proofs issued up to 5 minutes ahead (M13).
const DPOP_IAT_FUTURE_TOLERANCE_SECS: u64 = 30;

/// Seen DPoP proof `jti`s mapped to the time they can be forgotten (the past
/// acceptance horizon). In-memory and per-process — RFC 9449 §11.1 permits
/// storing the `jti` for the window in which the proof would be accepted. A
/// multi-instance deployment behind a load balancer would need a shared store
/// for cross-instance replay protection; single-instance is fully covered.
static DPOP_JTI_SEEN: LazyLock<Mutex<HashMap<String, u64>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Reject a DPoP proof whose `jti` has already been seen within its acceptance
/// window (replay). On first sight the `jti` is recorded until `expires_at`.
///
/// The cache key is scoped by the proof's JWK thumbprint so that a `jti`
/// collision between two distinct keys/clients cannot false-positive as a
/// replay — a true replay reuses the same proof key. `thumbprint` is base64url
/// (no `:`), so the join is unambiguous.
fn check_and_record_jti(
    thumbprint: &str,
    jti: &str,
    expires_at: u64,
    now: u64,
) -> Result<(), AppError> {
    let key = format!("{thumbprint}:{jti}");
    let mut seen = DPOP_JTI_SEEN.lock().unwrap_or_else(|e| e.into_inner());
    if seen.get(&key).is_some_and(|&exp| exp > now) {
        return Err(AppError::Auth("DPoP proof replay detected".into()));
    }
    // Bound memory: drop entries past their horizon once the map grows large.
    if seen.len() > 10_000 {
        seen.retain(|_, &mut exp| exp > now);
    }
    seen.insert(key, expires_at);
    Ok(())
}

#[derive(Debug, Deserialize)]
struct DpopHeader {
    alg: String,
    typ: String,
    jwk: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct DpopPayload {
    htm: String,
    htu: String,
    iat: u64,
    ath: Option<String>,
    jti: String,
}

/// Extract the JWK thumbprint from a DPoP proof JWT header without full validation.
pub fn extract_proof_thumbprint(proof_jwt: &str) -> Result<String, AppError> {
    let header_b64 = proof_jwt
        .split('.')
        .next()
        .ok_or_else(|| AppError::Auth("invalid DPoP proof format".into()))?;

    let header_bytes = URL_SAFE_NO_PAD
        .decode(header_b64)
        .map_err(|_| AppError::Auth("invalid DPoP proof header encoding".into()))?;

    let header: DpopHeader = serde_json::from_slice(&header_bytes)
        .map_err(|_| AppError::Auth("invalid DPoP proof header".into()))?;

    super::keys::compute_jwk_thumbprint(&header.jwk)
}

/// Validate a DPoP proof JWT.
///
/// Checks:
/// - `typ` is `dpop+jwt`
/// - `alg` is `ES256`
/// - `htm` matches the request method
/// - `htu` matches the request URL (scheme + host + path, no query/fragment)
/// - `iat` is within 5 minutes of now
/// - `ath` matches SHA256(access_token) if provided
/// - Signature is valid against the embedded JWK
/// - JWK thumbprint matches the expected thumbprint
pub fn validate_dpop_proof(
    proof_jwt: &str,
    expected_method: &str,
    expected_url: &str,
    access_token: &str,
    expected_thumbprint: &str,
) -> Result<(), AppError> {
    let parts: Vec<&str> = proof_jwt.split('.').collect();
    if parts.len() != 3 {
        return Err(AppError::Auth("invalid DPoP proof format".into()));
    }

    // Decode header
    let header_bytes = URL_SAFE_NO_PAD
        .decode(parts[0])
        .map_err(|_| AppError::Auth("invalid DPoP proof header encoding".into()))?;
    let header: DpopHeader = serde_json::from_slice(&header_bytes)
        .map_err(|_| AppError::Auth("invalid DPoP proof header".into()))?;

    // Check typ and alg
    if header.typ != "dpop+jwt" {
        return Err(AppError::Auth("DPoP proof typ must be dpop+jwt".into()));
    }
    if header.alg != "ES256" {
        return Err(AppError::Auth("DPoP proof alg must be ES256".into()));
    }

    // Decode payload
    let payload_bytes = URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|_| AppError::Auth("invalid DPoP proof payload encoding".into()))?;
    let payload: DpopPayload = serde_json::from_slice(&payload_bytes)
        .map_err(|_| AppError::Auth("invalid DPoP proof payload".into()))?;

    // Check htm
    if !payload.htm.eq_ignore_ascii_case(expected_method) {
        return Err(AppError::Auth("DPoP proof htm mismatch".into()));
    }

    // Check htu (strip query and fragment from expected URL for comparison)
    let expected_htu = strip_query_fragment(expected_url);
    if payload.htu != expected_htu {
        return Err(AppError::Auth("DPoP proof htu mismatch".into()));
    }

    // Check iat: a generous past tolerance for the proof to reach us, but only a
    // small future tolerance for clock skew.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    if payload.iat > now + DPOP_IAT_FUTURE_TOLERANCE_SECS {
        return Err(AppError::Auth("DPoP proof iat is in the future".into()));
    }
    if now > payload.iat + DPOP_IAT_PAST_TOLERANCE_SECS {
        return Err(AppError::Auth("DPoP proof expired".into()));
    }

    // Check ath (access token hash) — required per RFC 9449 section 4.2
    let expected_ath = URL_SAFE_NO_PAD.encode(Sha256::digest(access_token.as_bytes()));
    let ath = payload
        .ath
        .as_ref()
        .ok_or_else(|| AppError::Auth("DPoP proof missing required ath claim".into()))?;
    if *ath != expected_ath {
        return Err(AppError::Auth("DPoP proof ath mismatch".into()));
    }

    // Verify JWK thumbprint matches expected
    let proof_thumbprint = super::keys::compute_jwk_thumbprint(&header.jwk)?;
    if proof_thumbprint != expected_thumbprint {
        return Err(AppError::Auth(
            "DPoP proof key does not match session".into(),
        ));
    }

    // Verify signature
    let message = format!("{}.{}", parts[0], parts[1]);
    let sig_bytes = URL_SAFE_NO_PAD
        .decode(parts[2])
        .map_err(|_| AppError::Auth("invalid DPoP proof signature encoding".into()))?;

    verify_es256_jwk(&message, &sig_bytes, &header.jwk)?;

    // Replay protection (RFC 9449 §11.1): only after the proof is fully valid,
    // reject it if this `jti` was already used within its acceptance window.
    // Remember the `jti` until its `iat` falls outside the past tolerance (after
    // which the `iat` check alone would reject a replay).
    check_and_record_jti(
        &proof_thumbprint,
        &payload.jti,
        payload.iat + DPOP_IAT_PAST_TOLERANCE_SECS,
        now,
    )?;

    Ok(())
}

/// Verify an ES256 signature using a JWK public key.
fn verify_es256_jwk(
    message: &str,
    sig_bytes: &[u8],
    jwk: &serde_json::Value,
) -> Result<(), AppError> {
    let x_b64 = jwk["x"]
        .as_str()
        .ok_or_else(|| AppError::Auth("DPoP JWK missing x".into()))?;
    let y_b64 = jwk["y"]
        .as_str()
        .ok_or_else(|| AppError::Auth("DPoP JWK missing y".into()))?;

    let x_bytes = URL_SAFE_NO_PAD
        .decode(x_b64)
        .map_err(|_| AppError::Auth("invalid DPoP JWK x".into()))?;
    let y_bytes = URL_SAFE_NO_PAD
        .decode(y_b64)
        .map_err(|_| AppError::Auth("invalid DPoP JWK y".into()))?;

    // Build SEC1 uncompressed point: 0x04 || x || y
    let mut sec1 = Vec::with_capacity(1 + 32 + 32);
    sec1.push(0x04);
    sec1.extend_from_slice(&x_bytes);
    sec1.extend_from_slice(&y_bytes);

    let verifying_key = VerifyingKey::from_sec1_bytes(&sec1)
        .map_err(|_| AppError::Auth("invalid DPoP public key".into()))?;

    let signature = Signature::from_slice(sig_bytes)
        .map_err(|_| AppError::Auth("invalid DPoP signature format".into()))?;

    verifying_key
        .verify(message.as_bytes(), &signature)
        .map_err(|_| AppError::Auth("DPoP proof signature verification failed".into()))?;

    Ok(())
}

/// Strip query string and fragment from a URL (per RFC 9449 section 4.2).
fn strip_query_fragment(url: &str) -> &str {
    let end = url
        .find('#')
        .unwrap_or(url.len())
        .min(url.find('?').unwrap_or(url.len()));
    &url[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_query_fragment_works() {
        assert_eq!(
            strip_query_fragment("https://example.com/path"),
            "https://example.com/path"
        );
        assert_eq!(
            strip_query_fragment("https://example.com/path?query=1"),
            "https://example.com/path"
        );
        assert_eq!(
            strip_query_fragment("https://example.com/path#frag"),
            "https://example.com/path"
        );
        assert_eq!(
            strip_query_fragment("https://example.com/path?q=1#f"),
            "https://example.com/path"
        );
    }

    #[test]
    fn check_and_record_jti_rejects_replay_within_window() {
        // Unique jti per test to stay independent of the shared cache.
        let jti = "m13-unit-replay";
        let tp = "m13-unit-tp";
        assert!(check_and_record_jti(tp, jti, 1300, 1000).is_ok());
        // Same jti again while still within its window → replay.
        assert!(check_and_record_jti(tp, jti, 1300, 1000).is_err());
        // Once past the horizon it is accepted again.
        assert!(check_and_record_jti(tp, jti, 1700, 1400).is_ok());
    }

    #[test]
    fn check_and_record_jti_allows_distinct_jtis() {
        let tp = "m13-unit-distinct-tp";
        assert!(check_and_record_jti(tp, "m13-unit-distinct-a", 1300, 1000).is_ok());
        assert!(check_and_record_jti(tp, "m13-unit-distinct-b", 1300, 1000).is_ok());
    }

    #[test]
    fn check_and_record_jti_scopes_by_thumbprint() {
        // Same jti under two different thumbprints must not collide as a replay.
        let jti = "m13-unit-shared-jti";
        assert!(check_and_record_jti("m13-tp-alpha", jti, 1300, 1000).is_ok());
        assert!(check_and_record_jti("m13-tp-beta", jti, 1300, 1000).is_ok());
    }

    #[test]
    fn valid_proof_rejected_on_replay() {
        let keypair = crate::oauth::keys::generate_dpop_keypair().unwrap();
        let url = "https://example.com/xrpc/com.example.test";
        let proof = crate::oauth::pds_write::generate_dpop_proof(
            &keypair.private_jwk,
            "POST",
            url,
            "access-token",
            None,
        )
        .unwrap();

        // First use is accepted.
        assert!(
            validate_dpop_proof(&proof, "POST", url, "access-token", &keypair.thumbprint).is_ok()
        );
        // Replaying the exact same proof is rejected.
        let err = validate_dpop_proof(&proof, "POST", url, "access-token", &keypair.thumbprint)
            .unwrap_err();
        assert!(
            err.to_string().contains("replay"),
            "expected a replay error, got: {err}"
        );
    }

    #[test]
    fn rejects_invalid_format() {
        let result = validate_dpop_proof(
            "not.a.valid.jwt.too-many",
            "GET",
            "https://example.com",
            "token",
            "thumb",
        );
        assert!(result.is_err());
    }

    #[test]
    fn rejects_non_dpop_typ() {
        // Build a JWT with typ: "JWT" instead of "dpop+jwt"
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"ES256","typ":"JWT","jwk":{}}"#);
        let payload = URL_SAFE_NO_PAD
            .encode(r#"{"htm":"GET","htu":"https://example.com","iat":0,"jti":"x"}"#);
        let fake_sig = URL_SAFE_NO_PAD.encode(b"fakesig");
        let jwt = format!("{}.{}.{}", header, payload, fake_sig);

        let result = validate_dpop_proof(&jwt, "GET", "https://example.com", "token", "thumb");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("dpop+jwt"));
    }

    #[test]
    fn extract_proof_thumbprint_from_real_proof() {
        let keypair = crate::oauth::keys::generate_dpop_keypair().unwrap();

        let proof = crate::oauth::pds_write::generate_dpop_proof(
            &keypair.private_jwk,
            "POST",
            "https://pds.example.com/xrpc/test",
            "token",
            None,
        )
        .unwrap();

        let thumbprint = extract_proof_thumbprint(&proof).unwrap();
        assert_eq!(thumbprint, keypair.thumbprint);
    }

    #[test]
    fn extract_proof_thumbprint_rejects_garbage() {
        assert!(extract_proof_thumbprint("not-a-jwt").is_err());
    }

    #[test]
    fn extract_proof_thumbprint_rejects_bad_base64() {
        assert!(extract_proof_thumbprint("!!!.payload.sig").is_err());
    }

    #[test]
    fn extract_proof_thumbprint_different_keys_differ() {
        let kp1 = crate::oauth::keys::generate_dpop_keypair().unwrap();
        let kp2 = crate::oauth::keys::generate_dpop_keypair().unwrap();

        let proof1 = crate::oauth::pds_write::generate_dpop_proof(
            &kp1.private_jwk,
            "GET",
            "https://example.com",
            "t",
            None,
        )
        .unwrap();

        let proof2 = crate::oauth::pds_write::generate_dpop_proof(
            &kp2.private_jwk,
            "GET",
            "https://example.com",
            "t",
            None,
        )
        .unwrap();

        let t1 = extract_proof_thumbprint(&proof1).unwrap();
        let t2 = extract_proof_thumbprint(&proof2).unwrap();
        assert_ne!(t1, t2);
    }
}
