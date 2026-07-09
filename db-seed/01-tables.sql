-- ===============================================
-- 01‐tables.sql
-- ===============================================

CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ------------------------------------------------
-- Common updated_at trigger helper
-- ------------------------------------------------
CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = now();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ------------------------------------------------
-- 1) users table
-- ------------------------------------------------
CREATE TABLE users (
    id               TEXT         PRIMARY KEY DEFAULT ('usr_' || gen_random_uuid()::text),
    email            TEXT         NOT NULL,
    email_normalized TEXT         GENERATED ALWAYS AS (lower(email)) STORED,
    password_hash    TEXT         NOT NULL,
    display_name     TEXT,
    avatar_url       TEXT,
    role             TEXT         NOT NULL DEFAULT 'user'
                                     CHECK (role IN ('user', 'admin')),
    status           TEXT         NOT NULL DEFAULT 'active'
                                     CHECK (status IN ('active', 'disabled', 'deleted')),
    last_login_at    TIMESTAMPTZ,
    created_at       TIMESTAMPTZ  NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ  NOT NULL DEFAULT now(),
    CONSTRAINT uq_users_email_normalized UNIQUE (email_normalized)
);

CREATE INDEX idx_users_email_normalized ON users(email_normalized);

CREATE TRIGGER trg_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();


-- ------------------------------------------------
-- 2) devices table
-- ------------------------------------------------
CREATE TABLE devices (
    id           VARCHAR(36)    PRIMARY KEY,
    user_id      VARCHAR(36)    NOT NULL,
    name         VARCHAR(128)   NOT NULL,
    status       VARCHAR(32)    NOT NULL,
    device_os    VARCHAR(16)    NOT NULL,
    registered_at TIMESTAMPTZ   NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by   VARCHAR(36),
    created_at   TIMESTAMPTZ    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    modified_by  VARCHAR(36),
    modified_at  TIMESTAMPTZ    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    -- enforce unique (user_id, name)
    UNIQUE (user_id, name),

    -- foreign key → users.id
    FOREIGN KEY (user_id) REFERENCES users(id)
);

-- Index to speed up lookups by user_id
CREATE INDEX idx_devices_user_id ON devices(user_id);


-- ------------------------------------------------
-- 3) uploaded_files table
-- ------------------------------------------------
CREATE TABLE uploaded_files (
    id                VARCHAR(36)  PRIMARY KEY,
    user_id           VARCHAR(36)  NOT NULL,
    file_name         VARCHAR(128) NOT NULL,  -- stored/generated file name
    origin_file_name  VARCHAR(128) NOT NULL,  -- original filename from upload
    file_relative_path VARCHAR(256) NOT NULL,
    file_url          VARCHAR(256) NOT NULL,
    content_type      VARCHAR(64)  NOT NULL,
    file_size         BIGINT       NOT NULL,  -- use BIGINT for “unsigned” 
    file_type         VARCHAR(16)  NOT NULL,

    created_by        VARCHAR(36),
    created_at        TIMESTAMPTZ  NOT NULL DEFAULT CURRENT_TIMESTAMP,
    modified_by       VARCHAR(36),
    modified_at       TIMESTAMPTZ  NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- foreign key → users.id
    FOREIGN KEY (user_id) REFERENCES users(id)
);

-- If you want a composite key (e.g. one file_name per user), uncomment and adjust:
--   UNIQUE (user_id, file_name);


-- ------------------------------------------------
-- 4) refresh_tokens table
-- ------------------------------------------------
CREATE TABLE refresh_tokens (
    id           BIGSERIAL PRIMARY KEY,
    user_id      TEXT        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash   TEXT        NOT NULL,
    user_agent   TEXT,
    ip           INET,
    expires_at   TIMESTAMPTZ NOT NULL,
    revoked_at   TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT uq_refresh_token_hash UNIQUE (token_hash)
);

CREATE INDEX idx_refresh_tokens_user ON refresh_tokens(user_id);
CREATE INDEX idx_refresh_tokens_active
    ON refresh_tokens(user_id, expires_at)
    WHERE revoked_at IS NULL;


-- ------------------------------------------------
-- 5) user_tokens table
-- ------------------------------------------------
CREATE TABLE user_tokens (
    id           BIGSERIAL PRIMARY KEY,
    user_id      TEXT        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    purpose      TEXT        NOT NULL CHECK (purpose IN ('email_verify', 'password_reset')),
    token_hash   TEXT        NOT NULL UNIQUE,
    expires_at   TIMESTAMPTZ NOT NULL,
    used_at      TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_user_tokens_user_purpose ON user_tokens(user_id, purpose);
