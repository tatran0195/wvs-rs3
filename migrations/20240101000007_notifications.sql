-- Notifications
CREATE TABLE IF NOT EXISTS notifications (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    category        VARCHAR(50) NOT NULL,
    event_type      VARCHAR(100) NOT NULL,
    title           VARCHAR(255) NOT NULL,
    message         TEXT NOT NULL,
    payload         JSONB,
    priority        VARCHAR(20) DEFAULT 'normal',

    is_read         BOOLEAN DEFAULT FALSE,
    read_at         TIMESTAMPTZ,
    is_dismissed    BOOLEAN DEFAULT FALSE,

    actor_id        UUID REFERENCES users(id),
    resource_type   VARCHAR(50),
    resource_id     UUID,

    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at      TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_notif_user_unread ON notifications(user_id, is_read)
    WHERE NOT is_read;
CREATE INDEX IF NOT EXISTS idx_notif_user_time ON notifications(user_id, created_at DESC);