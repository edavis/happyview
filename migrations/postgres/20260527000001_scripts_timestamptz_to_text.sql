-- The scripts and dead_letter_scripts tables were created with TIMESTAMPTZ
-- columns, but sqlx's AnyPool driver does not support Postgres TIMESTAMPTZ.
-- Convert to TEXT to match every other table (see 20260318, 20260325).

-- scripts
ALTER TABLE scripts ALTER COLUMN created_at DROP DEFAULT;
ALTER TABLE scripts ALTER COLUMN created_at TYPE TEXT USING created_at::text;
ALTER TABLE scripts ALTER COLUMN updated_at DROP DEFAULT;
ALTER TABLE scripts ALTER COLUMN updated_at TYPE TEXT USING updated_at::text;

-- dead_letter_scripts
ALTER TABLE dead_letter_scripts ALTER COLUMN created_at DROP DEFAULT;
ALTER TABLE dead_letter_scripts ALTER COLUMN created_at TYPE TEXT USING created_at::text;
ALTER TABLE dead_letter_scripts ALTER COLUMN resolved_at TYPE TEXT USING resolved_at::text;
