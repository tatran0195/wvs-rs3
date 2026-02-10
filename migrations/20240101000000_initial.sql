-- Initial schema: Users + Core
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TYPE IF NOT EXISTS user_role AS ENUM ('admin', 'manager', 'creator', 'viewer');
CREATE TYPE IF NOT EXISTS user_status AS ENUM ('active', 'inactive', 'locked');

CREATE TABLE IF NOT EXISTS users (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    username        VARCHAR(100) UNIQUE NOT NULL,
    email           VARCHAR(255) UNIQUE,
    password_hash   TEXT NOT NULL,
    display_name    VARCHAR(255),
    role            user_role NOT NULL DEFAULT 'viewer',
    status          user_status NOT NULL DEFAULT 'active',
    failed_login_attempts INTEGER DEFAULT 0,
    locked_until    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_login_at   TIMESTAMPTZ,
    created_by      UUID REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_role ON users(role);
CREATE INDEX IF NOT EXISTS idx_users_status ON users(status);