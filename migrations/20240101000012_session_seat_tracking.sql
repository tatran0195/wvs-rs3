-- Pool snapshots
CREATE TABLE IF NOT EXISTS pool_snapshots (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    total_seats     INTEGER NOT NULL,
    checked_out     INTEGER NOT NULL,
    available       INTEGER NOT NULL,
    admin_reserved  INTEGER NOT NULL,
    active_sessions INTEGER NOT NULL,
    drift_detected  BOOLEAN DEFAULT FALSE,
    drift_detail    JSONB,
    source          VARCHAR(50) NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_pool_snap_time ON pool_snapshots(created_at DESC);