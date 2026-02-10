-- Shares
CREATE TYPE IF NOT EXISTS share_type AS ENUM ('public_link', 'private_link', 'user_share');

CREATE TABLE IF NOT EXISTS shares (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    share_type      share_type NOT NULL,

    resource_type   resource_type NOT NULL,
    resource_id     UUID NOT NULL,

    created_by      UUID NOT NULL REFERENCES users(id),

    token           VARCHAR(64) UNIQUE,
    password_hash   TEXT,

    shared_with     UUID REFERENCES users(id),

    permission      acl_permission NOT NULL DEFAULT 'viewer',
    allow_download  BOOLEAN DEFAULT TRUE,

    max_downloads   INTEGER,
    download_count  INTEGER DEFAULT 0,
    expires_at      TIMESTAMPTZ,

    is_active       BOOLEAN DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_accessed   TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_shares_token ON shares(token) WHERE token IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_shares_resource ON shares(resource_type, resource_id);
CREATE INDEX IF NOT EXISTS idx_shares_user ON shares(shared_with);
CREATE INDEX IF NOT EXISTS idx_shares_creator ON shares(created_by);