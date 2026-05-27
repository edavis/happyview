ALTER TABLE backfill_repos ADD COLUMN records_fetched INTEGER NOT NULL DEFAULT 0;
CREATE INDEX IF NOT EXISTS idx_backfill_repos_job_status ON backfill_repos (job_id, status);
