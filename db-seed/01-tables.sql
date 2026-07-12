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

-- ------------------------------------------------
-- 6) dictionaries table
-- ------------------------------------------------
CREATE TABLE dictionaries (
    id                  TEXT PRIMARY KEY,
    name                TEXT NOT NULL,
    description         TEXT NOT NULL DEFAULT '',
    category            TEXT NOT NULL,
    tags                TEXT[] NOT NULL DEFAULT '{}',
    length              INTEGER NOT NULL DEFAULT 0 CHECK (length >= 0),
    language            TEXT NOT NULL,
    language_category   TEXT NOT NULL,
    default_pron_index  INTEGER,
    sort_order          INTEGER NOT NULL DEFAULT 0,
    is_published        BOOLEAN NOT NULL DEFAULT true,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT chk_dict_language CHECK (language IN (
        'en', 'romaji', 'zh', 'ja', 'code', 'de', 'kk', 'hapin', 'id'
    )),
    CONSTRAINT chk_dict_language_category CHECK (language_category IN (
        'en', 'ja', 'de', 'code', 'kk', 'id'
    ))
);

ALTER TABLE dictionaries
    ADD COLUMN chapter_count INTEGER GENERATED ALWAYS AS (
        CASE WHEN length = 0 THEN 0 ELSE CEIL(length / 20.0)::INTEGER END
    ) STORED;

CREATE TRIGGER trg_dictionaries_updated_at
    BEFORE UPDATE ON dictionaries
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE INDEX idx_dictionaries_lang_cat ON dictionaries(language_category);
CREATE INDEX idx_dictionaries_category ON dictionaries(category);
CREATE INDEX idx_dictionaries_published ON dictionaries(is_published, sort_order);
CREATE INDEX idx_dictionaries_tags ON dictionaries USING GIN (tags);
CREATE INDEX idx_dictionaries_name_trgm ON dictionaries(name);

-- ------------------------------------------------
-- 7) words table
-- ------------------------------------------------
CREATE TABLE words (
    id          BIGSERIAL PRIMARY KEY,
    dict_id     TEXT NOT NULL REFERENCES dictionaries(id) ON DELETE CASCADE,
    idx         INTEGER NOT NULL CHECK (idx >= 0),
    name        TEXT NOT NULL,
    trans       TEXT[] NOT NULL DEFAULT '{}',
    usphone     TEXT NOT NULL DEFAULT '',
    ukphone     TEXT NOT NULL DEFAULT '',
    notation    TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_words_dict_idx UNIQUE (dict_id, idx)
);

CREATE TRIGGER trg_words_updated_at
    BEFORE UPDATE ON words
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE INDEX idx_words_dict_idx ON words(dict_id, idx);
CREATE INDEX idx_words_dict_name ON words(dict_id, name);

CREATE OR REPLACE FUNCTION sync_dictionary_length(p_dict_id TEXT)
RETURNS INTEGER AS $$
DECLARE
    n INTEGER;
BEGIN
    SELECT COUNT(*)::INTEGER INTO n FROM words WHERE dict_id = p_dict_id;
    UPDATE dictionaries SET length = n, updated_at = now() WHERE id = p_dict_id;
    RETURN n;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION get_chapter_words(p_dict_id TEXT, p_chapter INTEGER)
RETURNS TABLE (
    idx INTEGER,
    name TEXT,
    trans TEXT[],
    usphone TEXT,
    ukphone TEXT,
    notation TEXT
) AS $$
BEGIN
    IF p_chapter < 0 THEN
        RAISE EXCEPTION 'chapter must be >= 0';
    END IF;
    RETURN QUERY
    SELECT w.idx, w.name, w.trans, w.usphone, w.ukphone, w.notation
      FROM words w
     WHERE w.dict_id = p_dict_id
       AND w.idx >= p_chapter * 20
       AND w.idx < (p_chapter + 1) * 20
     ORDER BY w.idx;
END;
$$ LANGUAGE plpgsql STABLE;

-- ------------------------------------------------
-- 8) word_records table
-- ------------------------------------------------
CREATE TABLE word_records (
    id            BIGSERIAL PRIMARY KEY,
    user_id       TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    word          TEXT NOT NULL,
    dict          TEXT NOT NULL,
    chapter       INTEGER,
    timing        DOUBLE PRECISION[] NOT NULL DEFAULT '{}',
    wrong_count   INTEGER NOT NULL DEFAULT 0 CHECK (wrong_count >= 0),
    mistakes      JSONB NOT NULL DEFAULT '{}'::jsonb,
    time_stamp    BIGINT NOT NULL,
    client_id     TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX uq_word_records_client
    ON word_records(user_id, client_id)
    WHERE client_id IS NOT NULL;
CREATE INDEX idx_word_records_user_time
    ON word_records(user_id, time_stamp);
CREATE INDEX idx_word_records_user_dict_chapter
    ON word_records(user_id, dict, chapter);
CREATE INDEX idx_word_records_user_word_dict
    ON word_records(user_id, word, dict);
CREATE INDEX idx_word_records_wrong
    ON word_records(user_id, dict, wrong_count)
    WHERE wrong_count > 0;
CREATE INDEX idx_word_records_mistakes
    ON word_records USING GIN (mistakes);

-- ------------------------------------------------
-- 9) chapter_records table
-- ------------------------------------------------
CREATE TABLE chapter_records (
    id                    BIGSERIAL PRIMARY KEY,
    user_id               TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    dict                  TEXT NOT NULL,
    chapter               INTEGER,
    time_stamp            BIGINT NOT NULL,
    time_seconds          INTEGER NOT NULL CHECK (time_seconds >= 0),
    correct_count         INTEGER NOT NULL DEFAULT 0,
    wrong_count           INTEGER NOT NULL DEFAULT 0,
    word_count            INTEGER NOT NULL DEFAULT 0,
    correct_word_indexes  INTEGER[] NOT NULL DEFAULT '{}',
    word_number           INTEGER NOT NULL DEFAULT 0,
    word_record_ids       BIGINT[] NOT NULL DEFAULT '{}',
    client_id             TEXT,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX uq_chapter_records_client
    ON chapter_records(user_id, client_id)
    WHERE client_id IS NOT NULL;
CREATE INDEX idx_chapter_records_user_dict_chapter
    ON chapter_records(user_id, dict, chapter);
CREATE INDEX idx_chapter_records_user_time
    ON chapter_records(user_id, time_stamp);

-- ------------------------------------------------
-- 10) review_records table
-- ------------------------------------------------
CREATE TABLE review_records (
    id           BIGSERIAL PRIMARY KEY,
    user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    dict         TEXT NOT NULL,
    idx          INTEGER NOT NULL DEFAULT 0 CHECK (idx >= 0),
    create_time  BIGINT NOT NULL,
    is_finished  BOOLEAN NOT NULL DEFAULT false,
    words        JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TRIGGER trg_review_records_updated_at
    BEFORE UPDATE ON review_records
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE INDEX idx_review_records_user_dict
    ON review_records(user_id, dict, create_time DESC);
CREATE INDEX idx_review_records_unfinished
    ON review_records(user_id, dict, create_time DESC)
    WHERE is_finished = false;

COMMENT ON TABLE review_records IS '智能复习会话；对应 IReviewRecord / generateNewWordReviewRecord';
COMMENT ON COLUMN review_records.words IS '按 errorCount*0.6 + latestErrorTime*0.4 排序后的 Word[]';

-- ------------------------------------------------
-- 11) review aggregation and ranking
-- ------------------------------------------------
CREATE OR REPLACE VIEW v_error_words AS
SELECT
    wr.user_id,
    wr.dict,
    wr.word,
    SUM(wr.wrong_count)::INTEGER AS wrong_count,
    MAX(wr.time_stamp) AS latest_error_time,
    COUNT(*)::INTEGER AS record_count,
    (ARRAY_AGG(wr.mistakes ORDER BY wr.time_stamp DESC))[1] AS latest_mistakes
FROM word_records wr
WHERE wr.wrong_count > 0
GROUP BY wr.user_id, wr.dict, wr.word;

COMMENT ON VIEW v_error_words IS '错题本聚合；对应 GET /error-book、TErrorWordData';

CREATE OR REPLACE FUNCTION rank_review_words(p_user_id TEXT, p_dict TEXT)
RETURNS TABLE (word TEXT, error_count INTEGER, latest_error_time BIGINT, score DOUBLE PRECISION) AS $$
BEGIN
    RETURN QUERY
    WITH base AS (
        SELECT
            e.word,
            e.wrong_count AS error_count,
            e.latest_error_time
        FROM v_error_words e
        WHERE e.user_id = p_user_id AND e.dict = p_dict
    ),
    ranked AS (
        SELECT
            b.*,
            RANK() OVER (ORDER BY b.error_count ASC)::DOUBLE PRECISION AS error_count_score,
            RANK() OVER (ORDER BY b.latest_error_time ASC)::DOUBLE PRECISION AS latest_error_time_score
        FROM base b
    )
    SELECT
        r.word,
        r.error_count,
        r.latest_error_time,
        (r.error_count_score * 0.6 + r.latest_error_time_score * 0.4) AS score
    FROM ranked r
    ORDER BY score ASC;
END;
$$ LANGUAGE plpgsql STABLE;
