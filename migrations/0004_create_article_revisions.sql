CREATE TABLE article_revisions (
    id BIGSERIAL PRIMARY KEY,
    article_id BIGINT NOT NULL REFERENCES articles(id) ON DELETE CASCADE,
    version INTEGER NOT NULL,
    title TEXT NOT NULL,
    slug CITEXT NOT NULL,
    body TEXT NOT NULL,
    published BOOLEAN NOT NULL,
    published_at TIMESTAMPTZ,
    author_id BIGINT NOT NULL REFERENCES users(id),
    edited_by BIGINT REFERENCES users(id),
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT article_revisions_article_version_key UNIQUE (article_id, version)
);

CREATE INDEX idx_article_revisions_article_version ON article_revisions (article_id, version DESC);
