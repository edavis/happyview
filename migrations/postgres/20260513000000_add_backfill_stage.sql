ALTER TABLE backfill_jobs ADD COLUMN stage TEXT NOT NULL DEFAULT 'pending';

UPDATE backfill_jobs SET stage = status WHERE status IN ('completed', 'failed');
UPDATE backfill_jobs SET stage = 'failed', status = 'failed', error = 'interrupted by restart' WHERE status = 'running';
