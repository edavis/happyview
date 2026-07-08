//! Constant-time comparison helpers for secrets and their hashes.
//!
//! Comparing secret material (or its digest) with `==` can leak how many
//! leading bytes matched via early-exit timing. These helpers compare in time
//! independent of the *content* of equal-length inputs. Input length is not
//! treated as secret — the values compared here are fixed-length hashes or
//! attacker-known challenges — so an early length-mismatch return is fine.

/// Constant-time equality over two byte slices. `subtle`'s slice comparison
/// short-circuits only on a length mismatch (length is not secret here); for
/// equal-length inputs it compares every byte regardless of where they differ.
pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    use subtle::ConstantTimeEq;
    a.ct_eq(b).into()
}

/// Constant-time equality over two strings (compares their UTF-8 bytes).
pub fn ct_eq_str(a: &str, b: &str) -> bool {
    ct_eq(a.as_bytes(), b.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_values_match() {
        assert!(ct_eq_str("a1b2c3", "a1b2c3"));
        assert!(ct_eq(b"\x00\x01\x02", b"\x00\x01\x02"));
    }

    #[test]
    fn different_same_length_do_not_match() {
        assert!(!ct_eq_str("a1b2c3", "a1b2c4"));
        // Differing only in the first byte must also be rejected.
        assert!(!ct_eq_str("X1b2c3", "a1b2c3"));
    }

    #[test]
    fn different_length_does_not_match() {
        assert!(!ct_eq_str("abc", "abcd"));
        assert!(!ct_eq_str("abcd", "abc"));
    }

    #[test]
    fn empty_values_match() {
        assert!(ct_eq_str("", ""));
    }
}
