DROP INDEX IF EXISTS idx_dpop_sessions_token_hash;
ALTER TABLE dpop_sessions DROP COLUMN access_token_hash;
