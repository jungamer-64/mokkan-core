ALTER TABLE articles
ADD COLUMN published_at TEXT NULL;

-- Backfill published_at for already published articles
UPDATE articles
SET published_at = created_at
WHERE published = 1 AND published_at IS NULL;
