use async_trait::async_trait;
use sqlx::PgPool;

use crate::domains::stats::domain::{
    model::{AnalysisWordRecord, ChapterStatsRow, StatsSummaryRow},
    repository::StatsRepository,
};

pub struct StatsRepo;

#[async_trait]
impl StatsRepository for StatsRepo {
    async fn is_active_user(&self, pool: &PgPool, user_id: &str) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM users WHERE id = $1 AND status = 'active')",
        )
        .bind(user_id)
        .fetch_one(pool)
        .await
    }

    async fn published_dictionary_chapter_count(
        &self,
        pool: &PgPool,
        dict_id: &str,
    ) -> Result<Option<i32>, sqlx::Error> {
        sqlx::query_scalar::<_, i32>(
            "SELECT chapter_count FROM dictionaries WHERE id = $1 AND is_published = true",
        )
        .bind(dict_id)
        .fetch_optional(pool)
        .await
    }

    async fn chapter_stats(
        &self,
        pool: &PgPool,
        user_id: &str,
        dict_id: &str,
        chapter: i32,
    ) -> Result<Option<ChapterStatsRow>, sqlx::Error> {
        sqlx::query_as::<_, ChapterStatsRow>(
            r#"
            SELECT exercise_count, avg_wrong_word_count, avg_wrong_input_count
              FROM v_chapter_stats
             WHERE user_id = $1 AND dict = $2 AND chapter = $3
            "#,
        )
        .bind(user_id)
        .bind(dict_id)
        .bind(chapter)
        .fetch_optional(pool)
        .await
    }

    async fn exercised_chapter_count(
        &self,
        pool: &PgPool,
        user_id: &str,
        dict_id: &str,
    ) -> Result<i32, sqlx::Error> {
        sqlx::query_scalar::<_, i32>(
            r#"
            SELECT COALESCE(
                (SELECT exercised_chapter_count
                   FROM v_dict_stats
                  WHERE user_id = $1 AND dict = $2),
                0
            )::INTEGER
            "#,
        )
        .bind(user_id)
        .bind(dict_id)
        .fetch_one(pool)
        .await
    }

    async fn summary(
        &self,
        pool: &PgPool,
        user_id: &str,
    ) -> Result<Option<StatsSummaryRow>, sqlx::Error> {
        sqlx::query_as::<_, StatsSummaryRow>(
            r#"
            SELECT word_record_count, chapter_record_count, total_time_seconds, first_practice_at
              FROM v_user_summary
             WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await
    }

    async fn first_word_record_at(
        &self,
        pool: &PgPool,
        user_id: &str,
    ) -> Result<Option<i64>, sqlx::Error> {
        sqlx::query_scalar::<_, Option<i64>>(
            "SELECT MIN(time_stamp) FROM word_records WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_one(pool)
        .await
    }

    async fn analysis_records(
        &self,
        pool: &PgPool,
        user_id: &str,
        from: i64,
        to: i64,
    ) -> Result<Vec<AnalysisWordRecord>, sqlx::Error> {
        sqlx::query_as::<_, AnalysisWordRecord>(
            r#"
            SELECT word, timing, wrong_count, mistakes, time_stamp
              FROM word_records
             WHERE user_id = $1 AND time_stamp >= $2 AND time_stamp <= $3
             ORDER BY time_stamp, id
            "#,
        )
        .bind(user_id)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await
    }
}
