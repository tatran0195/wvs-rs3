-- User session limits
CREATE TABLE IF NOT EXISTS user_session_limits (
    user_id         UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    max_sessions    INTEGER NOT NULL,
    reason          TEXT,
    set_by          UUID REFERENCES users(id),
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW()
);