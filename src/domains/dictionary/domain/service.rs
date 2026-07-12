use async_trait::async_trait;
use std::sync::Arc;

use crate::{
    common::error::AppError,
    domains::dictionary::dto::dictionary_dto::{
        DictionaryCreateDto, DictionaryDto, DictionaryListQuery, DictionaryListResponse,
        WordBulkRequest, WordBulkResponse, WordListQuery, WordListResponse, WordWithIndexDto,
    },
};

#[async_trait]
pub trait DictionaryServiceTrait: Send + Sync {
    fn create_service(pool: sqlx::PgPool) -> Arc<dyn DictionaryServiceTrait>
    where
        Self: Sized;

    async fn list(&self, query: DictionaryListQuery) -> Result<DictionaryListResponse, AppError>;
    async fn get(&self, id: String) -> Result<DictionaryDto, AppError>;
    async fn get_words(
        &self,
        id: String,
        query: WordListQuery,
    ) -> Result<WordListResponse, AppError>;
    async fn get_word(&self, id: String, word_name: String) -> Result<WordWithIndexDto, AppError>;
    async fn create(
        &self,
        user_id: String,
        payload: DictionaryCreateDto,
    ) -> Result<DictionaryDto, AppError>;
    async fn update(
        &self,
        user_id: String,
        id: String,
        payload: DictionaryCreateDto,
    ) -> Result<DictionaryDto, AppError>;
    async fn bulk_words(
        &self,
        user_id: String,
        id: String,
        payload: WordBulkRequest,
    ) -> Result<WordBulkResponse, AppError>;
    async fn delete(&self, user_id: String, id: String) -> Result<(), AppError>;
}
