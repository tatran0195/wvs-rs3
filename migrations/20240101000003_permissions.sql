-- Folders + Files + Versions + Chunked uploads + Permissions (ACL)
CREATE TABLE IF NOT EXISTS folders (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    storage_id  UUID NOT NULL REFERENCES storages(id) ON DELETE CASCADE,
    parent_id   UUID REFERENCES folders(id) ON DELETE CASCADE,
    name        VARCHAR(255) NOT NULL,
    path        TEXT NOT NULL,
    depth       INTEGER NOT NULL DEFAULT 0,

    owner_id    UUID NOT NULL REFERENCES users(id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(storage_id, path)
);

CREATE INDEX IF NOT EXISTS idx_folders_parent ON folders(parent_id);
CREATE INDEX IF NOT EXISTS idx_folders_path ON folders(storage_id, path);
CREATE INDEX IF NOT EXISTS idx_folders_owner ON folders(owner_id);

CREATE TABLE IF NOT EXISTS files (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    folder_id       UUID NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
    storage_id      UUID NOT NULL REFERENCES storages(id),

    name            VARCHAR(255) NOT NULL,
    storage_path    TEXT NOT NULL,
    mime_type       VARCHAR(255),
    size_bytes      BIGINT NOT NULL DEFAULT 0,
    checksum_sha256 VARCHAR(64),

    metadata        JSONB DEFAULT '{}',

    current_version INTEGER NOT NULL DEFAULT 1,
    is_locked       BOOLEAN DEFAULT FALSE,
    locked_by       UUID REFERENCES users(id),
    locked_at       TIMESTAMPTZ,

    owner_id        UUID NOT NULL REFERENCES users(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(folder_id, name)
);

CREATE INDEX IF NOT EXISTS idx_files_folder ON files(folder_id);
CREATE INDEX IF NOT EXISTS idx_files_storage ON files(storage_id);
CREATE INDEX IF NOT EXISTS idx_files_name ON files(name);
CREATE INDEX IF NOT EXISTS idx_files_mime ON files(mime_type);
CREATE INDEX IF NOT EXISTS idx_files_owner ON files(owner_id);
CREATE INDEX IF NOT EXISTS idx_files_search ON files USING gin(
    to_tsvector('english', name || ' ' || COALESCE(metadata->>'description', ''))
);

CREATE TABLE IF NOT EXISTS file_versions (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    file_id         UUID NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    version_number  INTEGER NOT NULL,
    storage_path    TEXT NOT NULL,
    size_bytes      BIGINT NOT NULL,
    checksum_sha256 VARCHAR(64),
    created_by      UUID NOT NULL REFERENCES users(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    comment         TEXT,

    UNIQUE(file_id, version_number)
);

CREATE TABLE IF NOT EXISTS chunked_uploads (
    id                  UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id             UUID NOT NULL REFERENCES users(id),
    storage_id          UUID NOT NULL REFERENCES storages(id),
    target_folder_id    UUID NOT NULL REFERENCES folders(id),

    file_name           VARCHAR(255) NOT NULL,
    file_size           BIGINT NOT NULL,
    mime_type           VARCHAR(255),
    chunk_size          INTEGER NOT NULL,
    total_chunks        INTEGER NOT NULL,
    uploaded_chunks     JSONB NOT NULL DEFAULT '[]',

    checksum_sha256     VARCHAR(64),
    temp_path           TEXT NOT NULL,

    status              VARCHAR(20) NOT NULL DEFAULT 'uploading',
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at          TIMESTAMPTZ NOT NULL,
    completed_at        TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_chunks_user ON chunked_uploads(user_id);
CREATE INDEX IF NOT EXISTS idx_chunks_status ON chunked_uploads(status);
CREATE INDEX IF NOT EXISTS idx_chunks_expiry ON chunked_uploads(expires_at)
    WHERE status = 'uploading';

-- ACL
CREATE TYPE IF NOT EXISTS resource_type AS ENUM ('file', 'folder', 'storage');
CREATE TYPE IF NOT EXISTS acl_permission AS ENUM ('owner', 'editor', 'commenter', 'viewer');
CREATE TYPE IF NOT EXISTS acl_inheritance AS ENUM ('inherit', 'block');

CREATE TABLE IF NOT EXISTS acl_entries (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    resource_type   resource_type NOT NULL,
    resource_id     UUID NOT NULL,

    user_id         UUID REFERENCES users(id) ON DELETE CASCADE,
    is_anyone       BOOLEAN DEFAULT FALSE,

    permission      acl_permission NOT NULL,
    inheritance     acl_inheritance NOT NULL DEFAULT 'inherit',

    granted_by      UUID NOT NULL REFERENCES users(id),
    expires_at      TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT acl_principal_check CHECK (
        (user_id IS NOT NULL AND NOT is_anyone) OR
        (user_id IS NULL AND is_anyone)
    )
);

CREATE INDEX IF NOT EXISTS idx_acl_resource ON acl_entries(resource_type, resource_id);
CREATE INDEX IF NOT EXISTS idx_acl_user ON acl_entries(user_id);
CREATE INDEX IF NOT EXISTS idx_acl_lookup ON acl_entries(resource_type, resource_id, user_id);