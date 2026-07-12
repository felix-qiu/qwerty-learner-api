use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;

use crate::{
    common::error::AppError,
    domains::error_book::dto::error_book_dto::{
        DictErrorWordsResponse, ErrorBookListQuery, ErrorBookListResponse,
    },
};

#[async_trait]
pub trait ErrorBookServiceTrait: Send + Sync {
    fn create_service(pool: PgPool) -> Arc<dyn ErrorBookServiceTrait>
    where
        Self: Sized;

    async fn list_error_book(
        &self,
        user_id: String,
        query: ErrorBookListQuery,
    ) -> Result<ErrorBookListResponse, AppError>;

    async fn get_dictionary_error_words(
        &self,
        user_id: String,
        dict_id: String,
    ) -> Result<DictErrorWordsResponse, AppError>;
}
