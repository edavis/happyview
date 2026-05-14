CREATE TABLE IF NOT EXISTS backfill_repos (
    job_id UUID NOT NULL REFERENCES backfill_jobs(id) ON DELETE CASCADE,
    did TEXT NOT NULL,
    pds_endpoint TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    PRIMARY KEY (job_id, did)
);
