-- Jobs
DO $$ BEGIN
    CREATE TYPE job_status AS ENUM (
        'pending', 'queued', 'running', 'completed', 'failed', 'cancelled'
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE job_priority AS ENUM ('low', 'normal', 'high', 'critical');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;


CREATE TABLE IF NOT EXISTS jobs (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    job_type        VARCHAR(100) NOT NULL,
    queue           VARCHAR(50) NOT NULL DEFAULT 'default',
    priority        job_priority NOT NULL DEFAULT 'normal',

    payload         JSONB NOT NULL,
    result          JSONB,
    error_message   TEXT,

    status          job_status NOT NULL DEFAULT 'pending',
    attempts        INTEGER DEFAULT 0,
    max_attempts    INTEGER DEFAULT 3,

    scheduled_at    TIMESTAMPTZ,
    started_at      TIMESTAMPTZ,
    completed_at    TIMESTAMPTZ,

    created_by      UUID REFERENCES users(id),
    worker_id       VARCHAR(100),

    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_jobs_status ON jobs(status, queue, priority);
CREATE INDEX IF NOT EXISTS idx_jobs_scheduled ON jobs(scheduled_at)
    WHERE status = 'pending';
CREATE INDEX IF NOT EXISTS idx_jobs_type ON jobs(job_type);