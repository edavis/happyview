//! Record CID content verification (security review finding L9).
//!
//! Records arriving from Jetstream and from PDS backfill carry a CID that was,
//! until now, trusted verbatim. For backfill the source PDS is attacker-
//! controllable (via the DID document), so a hostile PDS could serve a record
//! whose stored CID does not match its content. This module recomputes the CID
//! from the record's canonical DAG-CBOR encoding so a mismatch can be detected
//! and the record rejected before it is indexed.
//!
//! The canonical encoding (length-first map-key ordering, minimal integers,
//! CID links as CBOR tag 42, `$bytes` as byte strings) is delegated to
//! `serde_ipld_dagcbor` — the same DAG-CBOR codec the atproto Rust ecosystem
//! uses — rather than hand-rolled, so the recomputed CID matches what a PDS
//! produces. The only atproto-specific step is mapping the JSON `$link` /
//! `$bytes` conventions onto IPLD before encoding.

use cid::Cid;
use cid::multihash::Multihash;
use ipld_core::ipld::Ipld;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::str::FromStr;

/// DAG-CBOR IPLD codec.
const DAG_CBOR_CODEC: u64 = 0x71;
/// SHA2-256 multihash code.
const SHA2_256_CODE: u64 = 0x12;

/// Outcome of checking a claimed CID against a record's content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CidCheck {
    /// The claimed CID matches the CID recomputed from the record content.
    Match,
    /// The claimed CID is present but does not match the record content
    /// (malformed or content-mismatched) — the caller should reject the record.
    Mismatch,
    /// Verification was not attempted or not possible (no claimed CID, or the
    /// value could not be encoded to DAG-CBOR) — the caller should proceed
    /// without rejecting, to avoid dropping records over an encoder limitation.
    Skipped,
}

/// Convert an atproto JSON value into an IPLD value, honoring atproto's
/// `{"$link": "<cid>"}` (CID link) and `{"$bytes": "<base64>"}` (byte string)
/// conventions. Returns `None` if the value can't be represented (e.g. a
/// non-finite number, or a malformed `$link`/`$bytes`).
fn atproto_json_to_ipld(value: &Value) -> Option<Ipld> {
    Some(match value {
        Value::Null => Ipld::Null,
        Value::Bool(b) => Ipld::Bool(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ipld::Integer(i as i128)
            } else if let Some(u) = n.as_u64() {
                Ipld::Integer(u as i128)
            } else if let Some(f) = n.as_f64() {
                Ipld::Float(f)
            } else {
                return None;
            }
        }
        Value::String(s) => Ipld::String(s.clone()),
        Value::Array(arr) => {
            let mut items = Vec::with_capacity(arr.len());
            for v in arr {
                items.push(atproto_json_to_ipld(v)?);
            }
            Ipld::List(items)
        }
        Value::Object(obj) => {
            // atproto encodes a CID link as {"$link": "<cid>"} and raw bytes as
            // {"$bytes": "<base64>"} — single-key objects that must become an
            // IPLD Link / Bytes rather than a map.
            if obj.len() == 1 {
                if let Some(Value::String(link)) = obj.get("$link") {
                    return Some(Ipld::Link(Cid::from_str(link).ok()?));
                }
                if let Some(Value::String(b64)) = obj.get("$bytes") {
                    let bytes =
                        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64)
                            .ok()?;
                    return Some(Ipld::Bytes(bytes));
                }
            }
            let mut map = BTreeMap::new();
            for (k, v) in obj {
                map.insert(k.clone(), atproto_json_to_ipld(v)?);
            }
            Ipld::Map(map)
        }
    })
}

/// Recompute the DAG-CBOR CID (CIDv1, dag-cbor codec, sha2-256) for a record
/// value. `serde_ipld_dagcbor` produces the canonical encoding (length-first
/// key ordering, minimal integers, tag-42 links), matching what a PDS emits.
/// Returns `None` if the value cannot be represented as DAG-CBOR.
pub fn compute_record_cid(value: &Value) -> Option<Cid> {
    let ipld = atproto_json_to_ipld(value)?;
    let cbor = serde_ipld_dagcbor::to_vec(&ipld).ok()?;
    let digest = Sha256::digest(&cbor);

    // multihash: <code=0x12><len=0x20><digest>
    let mut mh_bytes = Vec::with_capacity(2 + digest.len());
    mh_bytes.push(SHA2_256_CODE as u8);
    mh_bytes.push(digest.len() as u8);
    mh_bytes.extend_from_slice(&digest);
    let multihash = Multihash::<64>::from_bytes(&mh_bytes).ok()?;

    Some(Cid::new_v1(DAG_CBOR_CODEC, multihash))
}

/// Check a claimed CID string against a record value.
///
/// - `Skipped` when there is no claimed CID, or the value can't be encoded to
///   DAG-CBOR (we never reject a record over an encoder limitation).
/// - `Mismatch` when the claimed CID is malformed, or is a valid CID that does
///   not match the recomputed content CID.
/// - `Match` otherwise.
pub fn verify_record_cid(claimed_cid: &str, value: &Value) -> CidCheck {
    if claimed_cid.is_empty() {
        return CidCheck::Skipped;
    }
    let computed = match compute_record_cid(value) {
        Some(cid) => cid,
        None => return CidCheck::Skipped,
    };
    match Cid::from_str(claimed_cid) {
        Ok(claimed) if claimed == computed => CidCheck::Match,
        _ => CidCheck::Mismatch,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Ground-truth CIDs, computed independently from the raw DAG-CBOR bytes
    // (see the L9 implementation notes): CIDv1, dag-cbor codec, sha2-256.
    const EMPTY_MAP_CID: &str = "bafyreigbtj4x7ip5legnfznufuopl4sg4knzc2cof6duas4b3q2fy6swua";
    const A1_CID: &str = "bafyreihltcnuuyqp2jm24aqydpnlj7b6w3ogwrplomrjtg5rifv44mmjey";
    // {"b":1,"aa":2} under length-first canonical key ordering. The naive
    // raw-string ordering ("aa" < "b") would give a DIFFERENT CID, so this
    // vector proves the encoder uses DAG-CBOR canonical ordering.
    const ORDERING_CID: &str = "bafyreihbaf6v4gjeo76rl6ncekrny5lwbgyjf7zdw2m7w77xsjm3xvige4";

    #[test]
    fn computes_known_cid_for_empty_map() {
        assert_eq!(
            compute_record_cid(&json!({}))
                .expect("encodable")
                .to_string(),
            EMPTY_MAP_CID
        );
    }

    #[test]
    fn computes_known_cid_for_small_record() {
        assert_eq!(
            compute_record_cid(&json!({ "a": 1 }))
                .expect("encodable")
                .to_string(),
            A1_CID
        );
    }

    #[test]
    fn uses_length_first_canonical_key_ordering() {
        // Input order is deliberately NOT the canonical order.
        assert_eq!(
            compute_record_cid(&json!({ "aa": 2, "b": 1 }))
                .expect("encodable")
                .to_string(),
            ORDERING_CID
        );
    }

    #[test]
    fn verify_matches_recomputed_cid() {
        let value = json!({
            "$type": "app.bsky.feed.post",
            "text": "hello",
            "createdAt": "2023-01-01T00:00:00.000Z"
        });
        let cid = compute_record_cid(&value).expect("encodable").to_string();
        assert_eq!(verify_record_cid(&cid, &value), CidCheck::Match);
    }

    #[test]
    fn verify_detects_content_mismatch() {
        // A structurally valid CID that belongs to a different value.
        assert_eq!(
            verify_record_cid(EMPTY_MAP_CID, &json!({ "text": "hello" })),
            CidCheck::Mismatch
        );
    }

    #[test]
    fn verify_treats_unparseable_claimed_cid_as_mismatch() {
        assert_eq!(
            verify_record_cid("not-a-real-cid", &json!({ "text": "hi" })),
            CidCheck::Mismatch
        );
    }

    #[test]
    fn verify_skips_when_no_claimed_cid() {
        assert_eq!(
            verify_record_cid("", &json!({ "text": "hi" })),
            CidCheck::Skipped
        );
    }

    #[test]
    fn link_encodes_as_ipld_link_not_string() {
        // {"$link": cid} must encode as a CBOR tag-42 link, so it produces a
        // different CID than the same CID carried as a plain string.
        let as_link = json!({ "ref": { "$link": EMPTY_MAP_CID } });
        let as_string = json!({ "ref": EMPTY_MAP_CID });
        assert_ne!(
            compute_record_cid(&as_link).expect("encodable"),
            compute_record_cid(&as_string).expect("encodable"),
        );
    }

    #[test]
    fn bytes_encode_as_byte_string_not_text() {
        // {"$bytes": base64} must encode as a CBOR byte string.
        let as_bytes = json!({ "data": { "$bytes": "aGVsbG8=" } }); // "hello"
        let as_string = json!({ "data": "aGVsbG8=" });
        assert_ne!(
            compute_record_cid(&as_bytes).expect("encodable"),
            compute_record_cid(&as_string).expect("encodable"),
        );
    }
}
