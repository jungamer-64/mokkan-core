-- migrations/0003_add_full_text_search.sql
ALTER TABLE articles
ADD COLUMN IF NOT EXISTS search tsvector
    GENERATED ALWAYS AS (
        setweight(to_tsvector('simple', coalesce(title, '')), 'A') ||
        setweight(to_tsvector('simple', coalesce(body,  '')), 'B')
    ) STORED;

CREATE INDEX IF NOT EXISTS idx_articles_search ON articles USING GIN (search);
