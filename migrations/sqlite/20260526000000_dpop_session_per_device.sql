DROP INDEX idx_dpop_sessions_client_user;
CREATE UNIQUE INDEX idx_dpop_sessions_client_user_key ON dpop_sessions(api_client_id, user_did, dpop_key_id);
