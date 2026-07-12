use async_trait::async_trait;
use sqlx::{PgPool, Postgres, Transaction};

use crate::domains::record::domain::{
    model::{ChapterRecord, NewChapterRecord, NewWordRecord, WordRecord},
    repository::{ChapterFilter, ChapterRecordFilter, RecordRepository, WordRecordFilter},
};

pub struct RecordRepo;

const WORD_RECORD_COLUMNS: &str = r#"
    id, word, dict, chapter, timing, wrong_count, mistakes, time_stamp
"#;

const CHAPTER_RECORD_COLUMNS: &str = r#"
    id, dict, chapter, time_stamp, time_seconds, correct_count,
    wrong_count, word_count, correct_word_indexes, word_number,
    word_record_ids
"#;

#[async_trait]
impl RecordRepository for RecordRepo {
    async fn is_active_user(&self, pool: &PgPool, user_id: &str) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM users WHERE id = $1 AND status = 'active')",
        )
        .bind(user_id)
        .fetch_one(pool)
        .await
    }

    async fn create_word_record(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        record: NewWordRecord,
    ) -> Result<WordRecord, sqlx::Error> {
        sqlx::query_as::<_, WordRecord>(&format!(
            r#"
            INSERT INTO word_records
                (user_id, word, dict, chapter, timing, wrong_count, mistakes, time_stamp)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING {WORD_RECORD_COLUMNS}
            "#
        ))
        .bind(record.user_id)
        .bind(record.word)
        .bind(record.dict)
        .bind(record.chapter)
        .bind(record.timing)
        .bind(record.wrong_count)
        .bind(record.mistakes)
        .bind(record.time_stamp)
        .fetch_one(&mut **tx)
        .await
    }

    async fn list_word_records(
        &self,
        pool: &PgPool,
        user_id: &str,
        filter: &WordRecordFilter,
    ) -> Result<(Vec<WordRecord>, i64), sqlx::Error> {
        let direction = if filter.ascending { "ASC" } else { "DESC" };
        let (chapter_mode, chapter_value) = chapter_filter_parts(filter.chapter);
        let records = sqlx::query_as::<_, WordRecord>(&format!(
            r#"
            SELECT {WORD_RECORD_COLUMNS}
              FROM word_records
             WHERE user_id = $1
               AND ($2::text IS NULL OR dict = $2)
               AND ($3 = 0 OR ($3 = 1 AND chapter IS NULL) OR ($3 = 2 AND chapter = $4))
               AND ($5::text IS NULL OR word = $5)
               AND (NOT $6 OR wrong_count > 0)
               AND ($7::bigint IS NULL OR time_stamp >= $7)
               AND ($8::bigint IS NULL OR time_stamp <= $8)
             ORDER BY time_stamp {direction}, id {direction}
             OFFSET $9
             LIMIT $10
            "#
        ))
        .bind(user_id)
        .bind(filter.dict.as_deref())
        .bind(chapter_mode)
        .bind(chapter_value)
        .bind(filter.word.as_deref())
        .bind(filter.wrong_only)
        .bind(filter.from)
        .bind(filter.to)
        .bind(filter.offset)
        .bind(filter.limit)
        .fetch_all(pool)
        .await?;

        let total = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
              FROM word_records
             WHERE user_id = $1
               AND ($2::text IS NULL OR dict = $2)
               AND ($3 = 0 OR ($3 = 1 AND chapter IS NULL) OR ($3 = 2 AND chapter = $4))
               AND ($5::text IS NULL OR word = $5)
               AND (NOT $6 OR wrong_count > 0)
               AND ($7::bigint IS NULL OR time_stamp >= $7)
               AND ($8::bigint IS NULL OR time_stamp <= $8)
            "#,
        )
        .bind(user_id)
        .bind(filter.dict.as_deref())
        .bind(chapter_mode)
        .bind(chapter_value)
        .bind(filter.word.as_deref())
        .bind(filter.wrong_only)
        .bind(filter.from)
        .bind(filter.to)
        .fetch_one(pool)
        .await?;

        Ok((records, total))
    }

    async fn delete_word_records(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user_id: &str,
        word: &str,
        dict: &str,
    ) -> Result<u64, sqlx::Error> {
        let result =
            sqlx::query("DELETE FROM word_records WHERE user_id = $1 AND word = $2 AND dict = $3")
                .bind(user_id)
                .bind(word)
                .bind(dict)
                .execute(&mut **tx)
                .await?;
        Ok(result.rows_affected())
    }

    async fn create_chapter_record(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        record: NewChapterRecord,
    ) -> Result<ChapterRecord, sqlx::Error> {
        sqlx::query_as::<_, ChapterRecord>(&format!(
            r#"
            INSERT INTO chapter_records
                (user_id, dict, chapter, time_stamp, time_seconds, correct_count,
                 wrong_count, word_count, correct_word_indexes, word_number, word_record_ids)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING {CHAPTER_RECORD_COLUMNS}
            "#
        ))
        .bind(record.user_id)
        .bind(record.dict)
        .bind(record.chapter)
        .bind(record.time_stamp)
        .bind(record.time_seconds)
        .bind(record.correct_count)
        .bind(record.wrong_count)
        .bind(record.word_count)
        .bind(record.correct_word_indexes)
        .bind(record.word_number)
        .bind(record.word_record_ids)
        .fetch_one(&mut **tx)
        .await
    }

    async fn list_chapter_records(
        &self,
        pool: &PgPool,
        user_id: &str,
        filter: &ChapterRecordFilter,
    ) -> Result<(Vec<ChapterRecord>, i64), sqlx::Error> {
        let (chapter_mode, chapter_value) = chapter_filter_parts(filter.chapter);
        let records = sqlx::query_as::<_, ChapterRecord>(&format!(
            r#"
            SELECT {CHAPTER_RECORD_COLUMNS}
              FROM chapter_records
             WHERE user_id = $1
               AND ($2::text IS NULL OR dict = $2)
               AND ($3 = 0 OR ($3 = 1 AND chapter IS NULL) OR ($3 = 2 AND chapter = $4))
               AND ($5::bigint IS NULL OR time_stamp >= $5)
               AND ($6::bigint IS NULL OR time_stamp <= $6)
             ORDER BY time_stamp DESC, id DESC
             OFFSET $7
             LIMIT $8
            "#
        ))
        .bind(user_id)
        .bind(filter.dict.as_deref())
        .bind(chapter_mode)
        .bind(chapter_value)
        .bind(filter.from)
        .bind(filter.to)
        .bind(filter.offset)
        .bind(filter.limit)
        .fetch_all(pool)
        .await?;

        let total = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
              FROM chapter_records
             WHERE user_id = $1
               AND ($2::text IS NULL OR dict = $2)
               AND ($3 = 0 OR ($3 = 1 AND chapter IS NULL) OR ($3 = 2 AND chapter = $4))
               AND ($5::bigint IS NULL OR time_stamp >= $5)
               AND ($6::bigint IS NULL OR time_stamp <= $6)
            "#,
        )
        .bind(user_id)
        .bind(filter.dict.as_deref())
        .bind(chapter_mode)
        .bind(chapter_value)
        .bind(filter.from)
        .bind(filter.to)
        .fetch_one(pool)
        .await?;

        Ok((records, total))
    }
}

fn chapter_filter_parts(filter: ChapterFilter) -> (i32, Option<i32>) {
    match filter {
        ChapterFilter::Any => (0, None),
        ChapterFilter::Null => (1, None),
        ChapterFilter::Value(chapter) => (2, Some(chapter)),
    }
}
