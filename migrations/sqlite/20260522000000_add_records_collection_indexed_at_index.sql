CREATE INDEX IF NOT EXISTS idx_records_collection_indexed_at
ON records (collection, indexed_at DESC);
