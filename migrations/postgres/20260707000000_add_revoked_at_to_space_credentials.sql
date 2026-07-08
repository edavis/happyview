-- Add revocation support to space credentials (M3).
-- A NULL revoked_at means active; a timestamp means the credential (and any
-- other credential sharing the row) has been revoked and must be rejected.
ALTER TABLE happyview_space_credentials ADD COLUMN revoked_at TEXT;
