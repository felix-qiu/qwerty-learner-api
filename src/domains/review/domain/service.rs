use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;

use crate::{
    common::error::AppError,
    domains::review::dto::review_dto::{
        LatestReviewQuery, LatestReviewResponse, ReviewCreateDto, ReviewListQuery,
        ReviewListResponse, ReviewRecordDto, ReviewUpdateDto,
    },
};

#[async_trait]
pub trait ReviewServiceTrait: Send + Sync {
    fn create_service(pool: PgPool) -> Arc<dyn ReviewServiceTrait>
    where
        Self: Sized;

    async fn list_reviews(
        &self,
        user_id: String,
        query: ReviewListQuery,
    ) -> Result<ReviewListResponse, AppError>;

    async fn create_review(
        &self,
        user_id: String,
        payload: ReviewCreateDto,
    ) -> Result<ReviewRecordDto, AppError>;

    async fn latest_review(
        &self,
        user_id: String,
        query: LatestReviewQuery,
    ) -> Result<LatestReviewResponse, AppError>;

    async fn update_review(
        &self,
        user_id: String,
        review_id: i64,
        payload: ReviewUpdateDto,
    ) -> Result<ReviewRecordDto, AppError>;
}
