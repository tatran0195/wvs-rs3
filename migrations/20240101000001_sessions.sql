-- Sessions
DO $$ BEGIN
    CREATE TYPE presence_status AS ENUM ('active', 'idle', 'away', 'dnd', 'offline');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;


CREATE TABLE IF NOT EXISTS sessions (
    id                  UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id             UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash          TEXT NOT NULL,
    refresh_token_hash  TEXT,
    ip_address          INET NOT NULL,
    user_agent          TEXT,
    device_info         JSONB,

    license_checkout_id VARCHAR(255),
    seat_allocated_at   TIMESTAMPTZ,
    overflow_kicked     UUID REFERENCES sessions(id),

    presence_status     presence_status DEFAULT 'active',
    ws_connected        BOOLEAN DEFAULT FALSE,
    ws_connected_at     TIMESTAMPTZ,

    terminated_by       UUID REFERENCES users(id),
    terminated_reason   TEXT,
    terminated_at       TIMESTAMPTZ,

    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at          TIMESTAMPTZ NOT NULL,
    last_activity       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_active ON sessions(user_id, expires_at)
    WHERE terminated_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_sessions_expiry ON sessions(expires_at);