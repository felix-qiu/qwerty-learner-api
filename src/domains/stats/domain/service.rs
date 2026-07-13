use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;

use crate::{
    common::error::AppError,
    domains::stats::dto::stats_dto::{
        AnalysisStatsDto, AnalysisStatsQuery, ChapterStatsDto, ChapterStatsQuery, DictStatsDto,
        StatsSummaryDto,
    },
};

#[async_trait]
pub trait StatsServiceTrait: Send + Sync {
    fn create_service(pool: PgPool) -> Arc<dyn StatsServiceTrait>
    where
        Self: Sized;

    async fn get_chapter_stats(
        &self,
        user_id: String,
        query: ChapterStatsQuery,
    ) -> Result<ChapterStatsDto, AppError>;

    async fn get_dictionary_stats(
        &self,
        user_id: String,
        dict_id: String,
    ) -> Result<DictStatsDto, AppError>;

    async fn get_analysis_stats(
        &self,
        user_id: String,
        query: AnalysisStatsQuery,
    ) -> Result<AnalysisStatsDto, AppError>;

    async fn get_stats_summary(&self, user_id: String) -> Result<StatsSummaryDto, AppError>;
}
