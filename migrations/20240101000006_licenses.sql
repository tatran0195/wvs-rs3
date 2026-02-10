-- License checkouts
CREATE TABLE IF NOT EXISTS license_checkouts (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    session_id      UUID REFERENCES sessions(id) ON DELETE SET NULL,
    user_id         UUID NOT NULL REFERENCES users(id),
    feature_name    VARCHAR(100) NOT NULL,
    checkout_token  VARCHAR(255) NOT NULL,

    checked_out_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    checked_in_at   TIMESTAMPTZ,

    ip_address      INET,
    is_active       BOOLEAN DEFAULT TRUE
);

CREATE INDEX IF NOT EXISTS idx_lic_active ON license_checkouts(is_active)
    WHERE is_active = TRUE;
CREATE INDEX IF NOT EXISTS idx_lic_session ON license_checkouts(session_id);
CREATE INDEX IF NOT EXISTS idx_lic_user ON license_checkouts(user_id);