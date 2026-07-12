use async_trait::async_trait;
use sqlx::{PgPool, Postgres, Transaction};

use crate::domains::review::domain::{
    model::{NewReviewRecord, RankedReviewWord, ReviewRecord},
    repository::{ReviewListFilter, ReviewPatch, ReviewRepository},
};

pub struct ReviewRepo;

const REVIEW_COLUMNS: &str = "id, dict, idx, create_time, is_finished, words";

#[async_trait]
impl ReviewRepository for ReviewRepo {
    async fn is_active_user(&self, pool: &PgPool, user_id: &str) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM users WHERE id = $1 AND status = 'active')",
        )
        .bind(user_id)
        .fetch_one(pool)
        .await
    }

    async fn list(
        &self,
        pool: &PgPool,
        user_id: &str,
        filter: &ReviewListFilter,
    ) -> Result<(Vec<ReviewRecord>, i64), sqlx::Error> {
        let records = sqlx::query_as::<_, ReviewRecord>(&format!(
            r#"
            SELECT {REVIEW_COLUMNS}
              FROM review_records
             WHERE user_id = $1
               AND ($2::text IS NULL OR dict = $2)
             ORDER BY create_time DESC, id DESC
             OFFSET $3
             LIMIT $4
            "#
        ))
        .bind(user_id)
        .bind(filter.dict.as_deref())
        .bind(filter.offset)
        .bind(filter.limit)
        .fetch_all(pool)
        .await?;

        let total = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
              FROM review_records
             WHERE user_id = $1
               AND ($2::text IS NULL OR dict = $2)
            "#,
        )
        .bind(user_id)
        .bind(filter.dict.as_deref())
        .fetch_one(pool)
        .await?;

        Ok((records, total))
    }

    async fn latest(
        &self,
        pool: &PgPool,
        user_id: &str,
        dict: &str,
    ) -> Result<Option<ReviewRecord>, sqlx::Error> {
        sqlx::query_as::<_, ReviewRecord>(&format!(
            r#"
            SELECT {REVIEW_COLUMNS}
              FROM review_records
             WHERE user_id = $1 AND dict = $2
             ORDER BY create_time DESC, id DESC
             LIMIT 1
            "#
        ))
        .bind(user_id)
        .bind(dict)
        .fetch_optional(pool)
        .await
    }

    async fn rank_words(
        &self,
        pool: &PgPool,
        user_id: &str,
        dict: &str,
    ) -> Result<Vec<RankedReviewWord>, sqlx::Error> {
        sqlx::query_as::<_, RankedReviewWord>(
            r#"
            SELECT ranked.word AS name,
                   COALESCE(word.trans, ARRAY[]::text[]) AS trans,
                   COALESCE(word.usphone, '') AS usphone,
                   COALESCE(word.ukphone, '') AS ukphone,
                   word.notation
              FROM rank_review_words($1, $2) ranked
              LEFT JOIN LATERAL (
                    SELECT w.trans, w.usphone, w.ukphone, w.notation
                      FROM words w
                     WHERE w.dict_id = $2 AND w.name = ranked.word
                     ORDER BY w.idx
                     LIMIT 1
              ) word ON true
             ORDER BY ranked.score ASC,
                      ranked.error_count ASC,
                      ranked.latest_error_time ASC,
                      ranked.word ASC
            "#,
        )
        .bind(user_id)
        .bind(dict)
        .fetch_all(pool)
        .await
    }

    async fn create(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        record: NewReviewRecord,
    ) -> Result<ReviewRecord, sqlx::Error> {
        sqlx::query_as::<_, ReviewRecord>(&format!(
            r#"
            INSERT INTO review_records (user_id, dict, idx, create_time, is_finished, words)
            VALUES ($1, $2, 0, $3, false, $4)
            RETURNING {REVIEW_COLUMNS}
            "#
        ))
        .bind(record.user_id)
        .bind(record.dict)
        .bind(record.create_time)
        .bind(record.words)
        .fetch_one(&mut **tx)
        .await
    }

    async fn update(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user_id: &str,
        review_id: i64,
        patch: ReviewPatch,
    ) -> Result<Option<ReviewRecord>, sqlx::Error> {
        sqlx::query_as::<_, ReviewRecord>(&format!(
            r#"
            UPDATE review_records
               SET idx = COALESCE($3, idx),
                   is_finished = COALESCE($4, is_finished),
                   words = COALESCE($5, words)
             WHERE id = $1 AND user_id = $2
            RETURNING {REVIEW_COLUMNS}
            "#
        ))
        .bind(review_id)
        .bind(user_id)
        .bind(patch.index)
        .bind(patch.is_finished)
        .bind(patch.words)
        .fetch_optional(&mut **tx)
        .await
    }
}
