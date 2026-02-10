-- Storages
CREATE TYPE IF NOT EXISTS storage_provider_type AS ENUM ('local', 's3', 'webdav', 'smb');
CREATE TYPE IF NOT EXISTS storage_status AS ENUM ('active', 'inactive', 'error', 'syncing');

CREATE TABLE IF NOT EXISTS storages (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name            VARCHAR(255) NOT NULL,
    description     TEXT,
    provider_type   storage_provider_type NOT NULL,
    config          JSONB NOT NULL,
    status          storage_status NOT NULL DEFAULT 'active',
    is_default      BOOLEAN DEFAULT FALSE,

    quota_bytes     BIGINT,
    used_bytes      BIGINT DEFAULT 0,

    mount_path      VARCHAR(500),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_synced_at  TIMESTAMPTZ,
    created_by      UUID REFERENCES users(id)
);