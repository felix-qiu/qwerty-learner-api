use async_trait::async_trait;
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};

use crate::domains::review::domain::model::{NewReviewRecord, RankedReviewWord, ReviewRecord};

#[derive(Debug, Clone, Default)]
pub struct ReviewListFilter {
    pub dict: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone, Default)]
pub struct ReviewPatch {
    pub index: Option<i32>,
    pub is_finished: Option<bool>,
    pub words: Option<Value>,
}

#[async_trait]
pub trait ReviewRepository: Send + Sync {
    async fn is_active_user(&self, pool: &PgPool, user_id: &str) -> Result<bool, sqlx::Error>;

    async fn list(
        &self,
        pool: &PgPool,
        user_id: &str,
        filter: &ReviewListFilter,
    ) -> Result<(Vec<ReviewRecord>, i64), sqlx::Error>;

    async fn latest(
        &self,
        pool: &PgPool,
        user_id: &str,
        dict: &str,
    ) -> Result<Option<ReviewRecord>, sqlx::Error>;

    async fn rank_words(
        &self,
        pool: &PgPool,
        user_id: &str,
        dict: &str,
    ) -> Result<Vec<RankedReviewWord>, sqlx::Error>;

    async fn create(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        record: NewReviewRecord,
    ) -> Result<ReviewRecord, sqlx::Error>;

    async fn update(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user_id: &str,
        review_id: i64,
        patch: ReviewPatch,
    ) -> Result<Option<ReviewRecord>, sqlx::Error>;
}
