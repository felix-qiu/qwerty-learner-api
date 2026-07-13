-- =============================================================================
-- Qwerty Learner API — PostgreSQL Schema
-- =============================================================================
-- 要求：PostgreSQL 14+
-- 对齐前端：
--   src/typings/index.ts, src/typings/resource.ts
--   src/utils/db/record.ts, src/utils/db/index.ts
--   src/store/index.ts
-- 约定：
--   CHAPTER_LENGTH = 20
--   时间戳字段 *_stamp / create_time 使用 Unix 秒（UTC）
--   复习模式 chapter = -1
-- =============================================================================

CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ---------------------------------------------------------------------------
-- 公共：updated_at 自动刷新
-- ---------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = now();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- =============================================================================
-- 1. 用户与认证
-- =============================================================================

CREATE TABLE users (
  id              TEXT PRIMARY KEY DEFAULT ('usr_' || gen_random_uuid()::text),
  email           TEXT NOT NULL,
  email_normalized TEXT GENERATED ALWAYS AS (lower(email)) STORED,
  password_hash   TEXT NOT NULL,
  display_name    TEXT,
  avatar_url      TEXT,
  role            TEXT NOT NULL DEFAULT 'user'
                    CHECK (role IN ('user', 'admin')),
  status          TEXT NOT NULL DEFAULT 'active'
                    CHECK (status IN ('active', 'disabled', 'deleted')),
  last_login_at   TIMESTAMPTZ,
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  CONSTRAINT uq_users_email_normalized UNIQUE (email_normalized)
);

CREATE TRIGGER trg_users_updated_at
  BEFORE UPDATE ON users
  FOR EACH ROW EXECUTE FUNCTION set_updated_at();

COMMENT ON TABLE users IS '用户账号；替代前端无用户体系后的云同步主体';

-- Access / Refresh Token（refresh 落库，access 仅 JWT）
CREATE TABLE refresh_tokens (
  id           BIGSERIAL PRIMARY KEY,
  user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  token_hash   TEXT NOT NULL,                 -- SHA-256(refresh_token)
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

COMMENT ON TABLE refresh_tokens IS '刷新令牌；登出/轮换时标记 revoked_at';

-- 密码重置 / 邮箱验证（可选）
CREATE TABLE user_tokens (
  id           BIGSERIAL PRIMARY KEY,
  user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  purpose      TEXT NOT NULL CHECK (purpose IN ('email_verify', 'password_reset')),
  token_hash   TEXT NOT NULL UNIQUE,
  expires_at   TIMESTAMPTZ NOT NULL,
  used_at      TIMESTAMPTZ,
  created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_user_tokens_user_purpose ON user_tokens(user_id, purpose);

-- =============================================================================
-- 2. 词典与单词（替代 dictionary.ts + public/dicts/*.json）
-- =============================================================================

CREATE TABLE dictionaries (
  id                  TEXT PRIMARY KEY,       -- 如 cet4
  name                TEXT NOT NULL,
  description         TEXT NOT NULL DEFAULT '',
  category            TEXT NOT NULL,           -- 如「中国考试」「代码练习」
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

-- chapter_count = ceil(length / 20)，与前端 calcChapterCount 一致
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
CREATE INDEX idx_dictionaries_name_trgm ON dictionaries (name);  -- 若启用 pg_trgm 可改为 gin_trgm_ops

COMMENT ON TABLE dictionaries IS '词典元数据；替代 src/resources/dictionary.ts';
COMMENT ON COLUMN dictionaries.chapter_count IS 'GENERATED: ceil(length/20)';

CREATE TABLE words (
  id          BIGSERIAL PRIMARY KEY,
  dict_id     TEXT NOT NULL REFERENCES dictionaries(id) ON DELETE CASCADE,
  idx         INTEGER NOT NULL CHECK (idx >= 0),  -- 全局下标，从 0 起
  name        TEXT NOT NULL,
  trans       TEXT[] NOT NULL DEFAULT '{}',
  usphone     TEXT NOT NULL DEFAULT '',
  ukphone     TEXT NOT NULL DEFAULT '',
  notation    TEXT,                              -- 日语等注音
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  CONSTRAINT uq_words_dict_idx UNIQUE (dict_id, idx)
);

CREATE TRIGGER trg_words_updated_at
  BEFORE UPDATE ON words
  FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE INDEX idx_words_dict_idx ON words(dict_id, idx);
CREATE INDEX idx_words_dict_name ON words(dict_id, name);

COMMENT ON TABLE words IS '词典单词；替代 public/dicts/*.json';
COMMENT ON COLUMN words.idx IS '在整本词典中的全局下标，对应 WordWithIndex.index';

-- 批量写入后同步 dictionaries.length
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

-- 按章取词（CHAPTER_LENGTH = 20）
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
    AND w.idx <  (p_chapter + 1) * 20
  ORDER BY w.idx;
END;
$$ LANGUAGE plpgsql STABLE;

-- =============================================================================
-- 3. 练习记录（替代 Dexie wordRecords / chapterRecords）
-- =============================================================================

CREATE TABLE word_records (
  id            BIGSERIAL PRIMARY KEY,
  user_id       TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  word          TEXT NOT NULL,
  dict          TEXT NOT NULL,                 -- 词典 id；不强制 FK，兼容特殊场景
  chapter       INTEGER,                       -- NULL / >=0 / -1(复习)
  timing        DOUBLE PRECISION[] NOT NULL DEFAULT '{}',
  wrong_count   INTEGER NOT NULL DEFAULT 0 CHECK (wrong_count >= 0),
  mistakes      JSONB NOT NULL DEFAULT '{}'::jsonb,
  -- mistakes 示例: {"0":["x"],"2":["n","m"]}
  time_stamp    BIGINT NOT NULL,               -- Unix 秒
  client_id     TEXT,                          -- 离线同步幂等键（可选）
  created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- client_id 非空时幂等（允许多条 client_id IS NULL）
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

COMMENT ON TABLE word_records IS '单词练习记录；对应 IWordRecord / useSaveWordRecord';
COMMENT ON COLUMN word_records.timing IS '相邻字母输入时间差(ms)；totalTime = sum(timing)';
COMMENT ON COLUMN word_records.chapter IS '正常章节>=0；复习=-1；错题练习可为 NULL';
COMMENT ON COLUMN word_records.client_id IS '客户端生成的幂等 ID，用于 batch 同步去重';

CREATE TABLE chapter_records (
  id                    BIGSERIAL PRIMARY KEY,
  user_id               TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  dict                  TEXT NOT NULL,
  chapter               INTEGER,               -- 复习场景为 -1
  time_stamp            BIGINT NOT NULL,
  time_seconds          INTEGER NOT NULL CHECK (time_seconds >= 0),  -- 前端字段 time
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

COMMENT ON TABLE chapter_records IS '章节练习记录；对应 IChapterRecord / useSaveChapterRecord';
COMMENT ON COLUMN chapter_records.time_seconds IS '章节用时(秒)；wpm = round(word_count/time_seconds*60)';
COMMENT ON COLUMN chapter_records.correct_word_indexes IS '一次打对未犯错的单词下标';

-- =============================================================================
-- 4. 复习记录（替代 Dexie reviewRecords）
-- =============================================================================

CREATE TABLE review_records (
  id           BIGSERIAL PRIMARY KEY,
  user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  dict         TEXT NOT NULL,
  idx          INTEGER NOT NULL DEFAULT 0 CHECK (idx >= 0),  -- 前端字段 index
  create_time  BIGINT NOT NULL,              -- Unix 秒
  is_finished  BOOLEAN NOT NULL DEFAULT false,
  words        JSONB NOT NULL DEFAULT '[]'::jsonb,  -- Word[]
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

-- =============================================================================
-- 5. 用户设置（替代 Jotai atomWithStorage）
-- =============================================================================

-- 默认值与 src/store/index.ts 对齐
CREATE TABLE user_settings (
  user_id     TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  settings    JSONB NOT NULL DEFAULT '{
    "currentDict": "cet4",
    "currentChapter": 0,
    "loopWordConfig": { "times": 1 },
    "keySoundsConfig": {
      "isOpen": true,
      "isOpenClickSound": true,
      "volume": 1,
      "resource": null
    },
    "hintSoundsConfig": {
      "isOpen": true,
      "volume": 1,
      "isOpenWrongSound": true,
      "isOpenCorrectSound": true,
      "wrongResource": null,
      "correctResource": null
    },
    "pronunciation": {
      "isOpen": true,
      "volume": 1,
      "type": "us",
      "name": "美音",
      "isLoop": false,
      "isTransRead": false,
      "transVolume": 1,
      "rate": 1
    },
    "fontsize": { "foreignFont": 48, "translateFont": 18 },
    "randomConfig": { "isOpen": false },
    "phoneticConfig": { "isOpen": true, "type": "us" },
    "wordDictationConfig": {
      "isOpen": false,
      "type": "hideAll",
      "openBy": "auto"
    },
    "isShowPrevAndNextWord": true,
    "isIgnoreCase": true,
    "isShowAnswerOnHover": true,
    "isTextSelectable": false,
    "isOpenDarkMode": false,
    "dismissStartCardDate": null,
    "hasSeenEnhancedPromotion": false
  }'::jsonb,
  updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  CONSTRAINT chk_settings_is_object CHECK (jsonb_typeof(settings) = 'object')
);

CREATE TRIGGER trg_user_settings_updated_at
  BEFORE UPDATE ON user_settings
  FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE INDEX idx_user_settings_gin ON user_settings USING GIN (settings);

COMMENT ON TABLE user_settings IS '用户偏好；替代 src/store/index.ts 中 localStorage atoms';

-- 注册时自动创建默认设置
CREATE OR REPLACE FUNCTION create_default_user_settings()
RETURNS TRIGGER AS $$
BEGIN
  INSERT INTO user_settings (user_id) VALUES (NEW.id)
  ON CONFLICT (user_id) DO NOTHING;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_users_create_settings
  AFTER INSERT ON users
  FOR EACH ROW EXECUTE FUNCTION create_default_user_settings();

-- =============================================================================
-- 6. 音效资源目录（可选，供设置里 resource 引用）
-- =============================================================================

CREATE TABLE sound_resources (
  key         TEXT PRIMARY KEY,
  name        TEXT NOT NULL,
  filename    TEXT NOT NULL,
  kind        TEXT NOT NULL CHECK (kind IN ('key', 'correct', 'wrong')),
  is_default  BOOLEAN NOT NULL DEFAULT false,
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE sound_resources IS '键盘/对错提示音资源；对应 SoundResource';

-- =============================================================================
-- 7. 发音缓存（可选，/pronunciation 代理）
-- =============================================================================

CREATE TABLE pronunciation_cache (
  id           BIGSERIAL PRIMARY KEY,
  word         TEXT NOT NULL,
  pron_type    TEXT NOT NULL,              -- us/uk/ja/...
  content_type TEXT NOT NULL DEFAULT 'audio/mpeg',
  audio_bytes  BYTEA,
  source_url   TEXT,
  hit_count    INTEGER NOT NULL DEFAULT 0,
  expires_at   TIMESTAMPTZ,
  created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
  CONSTRAINT uq_pronunciation_word_type UNIQUE (word, pron_type)
);

CREATE TRIGGER trg_pronunciation_cache_updated_at
  BEFORE UPDATE ON pronunciation_cache
  FOR EACH ROW EXECUTE FUNCTION set_updated_at();

COMMENT ON TABLE pronunciation_cache IS '发音音频缓存；代理有道等上游';

-- =============================================================================
-- 8. 数据导入导出任务（可选审计）
-- =============================================================================

CREATE TABLE data_jobs (
  id            BIGSERIAL PRIMARY KEY,
  user_id       TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  job_type      TEXT NOT NULL CHECK (job_type IN ('export', 'import')),
  mode          TEXT CHECK (mode IN ('replace', 'merge')),
  status        TEXT NOT NULL DEFAULT 'pending'
                  CHECK (status IN ('pending', 'running', 'succeeded', 'failed')),
  include_parts TEXT[] NOT NULL DEFAULT '{wordRecords,chapterRecords,reviewRecords,settings}',
  file_size     BIGINT,
  word_count    INTEGER,
  chapter_count INTEGER,
  review_count  INTEGER,
  error_message TEXT,
  started_at    TIMESTAMPTZ,
  finished_at   TIMESTAMPTZ,
  created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_data_jobs_user ON data_jobs(user_id, created_at DESC);

COMMENT ON TABLE data_jobs IS '导入/导出任务审计；对应 POST /data/export|import';

-- =============================================================================
-- 9. 视图：错题本 / 统计（对应 ErrorBook、Stats API）
-- =============================================================================

-- 错题聚合（全局 / 按词典）
CREATE OR REPLACE VIEW v_error_words AS
SELECT
  wr.user_id,
  wr.dict,
  wr.word,
  SUM(wr.wrong_count)::INTEGER          AS wrong_count,
  MAX(wr.time_stamp)                    AS latest_error_time,
  COUNT(*)::INTEGER                     AS record_count,
  -- 合并 mistakes：应用层可再精细聚合；此处保留最近一条 mistakes 作参考
  (ARRAY_AGG(wr.mistakes ORDER BY wr.time_stamp DESC))[1] AS latest_mistakes
FROM word_records wr
WHERE wr.wrong_count > 0
GROUP BY wr.user_id, wr.dict, wr.word;

COMMENT ON VIEW v_error_words IS '错题本聚合；对应 GET /error-book、TErrorWordData';

-- 词典练习进度
CREATE OR REPLACE VIEW v_dict_stats AS
SELECT
  cr.user_id,
  cr.dict,
  COUNT(DISTINCT cr.chapter) FILTER (
    WHERE cr.chapter IS NOT NULL AND cr.chapter >= 0
  )::INTEGER AS exercised_chapter_count,
  COUNT(*)::INTEGER AS chapter_record_count,
  MAX(cr.time_stamp) AS last_practice_at
FROM chapter_records cr
GROUP BY cr.user_id, cr.dict;

COMMENT ON VIEW v_dict_stats IS '词典进度；对应 GET /stats/dictionaries/{id} / useDictStats';

-- 章节统计
CREATE OR REPLACE VIEW v_chapter_stats AS
SELECT
  cr.user_id,
  cr.dict,
  cr.chapter,
  COUNT(*)::INTEGER AS exercise_count,
  ROUND(
    AVG((cr.word_number - COALESCE(cardinality(cr.correct_word_indexes), 0))::NUMERIC),
    2
  )::DOUBLE PRECISION AS avg_wrong_word_count,
  ROUND(AVG(cr.wrong_count::NUMERIC), 2)::DOUBLE PRECISION AS avg_wrong_input_count
FROM chapter_records cr
GROUP BY cr.user_id, cr.dict, cr.chapter;

COMMENT ON VIEW v_chapter_stats IS '章节统计；对应 GET /stats/chapters / useChapterStats';

-- 用户总览
CREATE OR REPLACE VIEW v_user_summary AS
SELECT
  u.id AS user_id,
  (SELECT COUNT(*) FROM word_records wr WHERE wr.user_id = u.id)::INTEGER
    AS word_record_count,
  (SELECT COUNT(*) FROM chapter_records cr WHERE cr.user_id = u.id)::INTEGER
    AS chapter_record_count,
  (SELECT COALESCE(SUM(cr.time_seconds), 0) FROM chapter_records cr WHERE cr.user_id = u.id)::INTEGER
    AS total_time_seconds,
  (SELECT MIN(wr.time_stamp) FROM word_records wr WHERE wr.user_id = u.id)
    AS first_practice_at
FROM users u;

COMMENT ON VIEW v_user_summary IS '用户总览；对应 GET /stats/summary';

-- 按日练习原始聚合（分析页可在此基础上算 WPM / accuracy / level）
CREATE OR REPLACE VIEW v_daily_word_activity AS
SELECT
  wr.user_id,
  to_char(to_timestamp(wr.time_stamp) AT TIME ZONE 'Asia/Shanghai', 'YYYY-MM-DD') AS day,
  COUNT(*)::INTEGER AS exercise_time,
  COUNT(DISTINCT wr.word)::INTEGER AS unique_word_count,
  COUNT(*)::INTEGER AS word_count_raw,
  COALESCE(SUM(x.t), 0)::DOUBLE PRECISION AS total_timing_ms,
  COALESCE(SUM(wr.wrong_count), 0)::INTEGER AS wrong_count
FROM word_records wr
LEFT JOIN LATERAL (
  SELECT SUM(v)::DOUBLE PRECISION AS t
  FROM unnest(wr.timing) AS v
) x ON true
GROUP BY wr.user_id, day;

COMMENT ON VIEW v_daily_word_activity IS '按 Asia/Shanghai 自然日聚合；对应 useWordStats 的中间层';

-- =============================================================================
-- 10. 辅助函数：分析指标 / 复习排序
-- =============================================================================

-- 热力图 level：与前端 getLevel 一致
CREATE OR REPLACE FUNCTION activity_level(p_count INTEGER)
RETURNS INTEGER AS $$
BEGIN
  IF p_count IS NULL OR p_count <= 0 THEN RETURN 0; END IF;
  IF p_count < 4 THEN RETURN 1; END IF;
  IF p_count < 8 THEN RETURN 2; END IF;
  IF p_count < 12 THEN RETURN 3; END IF;
  RETURN 4;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- 从错题生成复习词序（errorCount 权重 0.6，latestErrorTime 权重 0.4）
-- 返回 word 名按复习优先级升序（错得少且久远的靠前）
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

-- =============================================================================
-- 11. 表清单（便于对照 API）
-- =============================================================================
-- users                 认证用户
-- refresh_tokens        刷新令牌
-- user_tokens           邮箱验证 / 密码重置
-- dictionaries          词典元数据          → GET/POST /dictionaries
-- words                 单词                → GET /dictionaries/{id}/words
-- word_records          单词练习记录        → /records/words
-- chapter_records       章节练习记录        → /records/chapters
-- review_records        复习会话            → /reviews
-- user_settings         用户设置            → /settings
-- sound_resources       音效资源目录
-- pronunciation_cache   发音缓存            → /pronunciation
-- data_jobs             导入导出审计        → /data/export|import
--
-- 视图：
-- v_error_words         错题本              → /error-book
-- v_dict_stats          词典进度            → /stats/dictionaries/{id}
-- v_chapter_stats       章节统计            → /stats/chapters
-- v_user_summary        用户总览            → /stats/summary
-- v_daily_word_activity 按日练习            → /stats/analysis
