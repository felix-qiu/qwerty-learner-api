use async_trait::async_trait;
use sqlx::PgPool;

use crate::domains::stats::domain::model::{AnalysisWordRecord, ChapterStatsRow, StatsSummaryRow};

#[async_trait]
pub trait StatsRepository: Send + Sync {
    async fn is_active_user(&self, pool: &PgPool, user_id: &str) -> Result<bool, sqlx::Error>;

    async fn published_dictionary_chapter_count(
        &self,
        pool: &PgPool,
        dict_id: &str,
    ) -> Result<Option<i32>, sqlx::Error>;

    async fn chapter_stats(
        &self,
        pool: &PgPool,
        user_id: &str,
        dict_id: &str,
        chapter: i32,
    ) -> Result<Option<ChapterStatsRow>, sqlx::Error>;

    async fn exercised_chapter_count(
        &self,
        pool: &PgPool,
        user_id: &str,
        dict_id: &str,
    ) -> Result<i32, sqlx::Error>;

    async fn summary(
        &self,
        pool: &PgPool,
        user_id: &str,
    ) -> Result<Option<StatsSummaryRow>, sqlx::Error>;

    async fn first_word_record_at(
        &self,
        pool: &PgPool,
        user_id: &str,
    ) -> Result<Option<i64>, sqlx::Error>;

    async fn analysis_records(
        &self,
        pool: &PgPool,
        user_id: &str,
        from: i64,
        to: i64,
    ) -> Result<Vec<AnalysisWordRecord>, sqlx::Error>;
}
