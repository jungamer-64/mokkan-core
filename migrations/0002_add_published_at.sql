ALTER TABLE articles
ADD COLUMN IF NOT EXISTS published_at TIMESTAMPTZ NULL;

-- Backfill published_at for already published articles
UPDATE articles
SET published_at = created_at
WHERE published = TRUE AND published_at IS NULL;
