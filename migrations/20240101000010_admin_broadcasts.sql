-- Admin broadcasts
CREATE TABLE IF NOT EXISTS admin_broadcasts (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    admin_id        UUID NOT NULL REFERENCES users(id),
    target          VARCHAR(100) NOT NULL,
    title           VARCHAR(255) NOT NULL,
    message         TEXT NOT NULL,
    severity        VARCHAR(20) NOT NULL,
    persistent      BOOLEAN DEFAULT FALSE,
    action_type     VARCHAR(50),
    action_payload  JSONB,
    delivered_count INTEGER DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);