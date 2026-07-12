use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;

use crate::{
    common::error::AppError,
    domains::record::dto::record_dto::{
        ChapterRecordCreateDto, ChapterRecordDto, ChapterRecordListQuery,
        ChapterRecordListResponse, DeleteWordRecordsQuery, DeleteWordRecordsResponse,
        WordRecordBatchRequest, WordRecordBatchResponse, WordRecordCreateDto, WordRecordDto,
        WordRecordListQuery, WordRecordListResponse,
    },
};

#[async_trait]
pub trait RecordServiceTrait: Send + Sync {
    fn create_service(pool: PgPool) -> Arc<dyn RecordServiceTrait>
    where
        Self: Sized;

    async fn create_word_record(
        &self,
        user_id: String,
        payload: WordRecordCreateDto,
    ) -> Result<WordRecordDto, AppError>;
    async fn batch_create_word_records(
        &self,
        user_id: String,
        payload: WordRecordBatchRequest,
    ) -> Result<WordRecordBatchResponse, AppError>;
    async fn list_word_records(
        &self,
        user_id: String,
        query: WordRecordListQuery,
    ) -> Result<WordRecordListResponse, AppError>;
    async fn delete_word_records(
        &self,
        user_id: String,
        query: DeleteWordRecordsQuery,
    ) -> Result<DeleteWordRecordsResponse, AppError>;
    async fn create_chapter_record(
        &self,
        user_id: String,
        payload: ChapterRecordCreateDto,
    ) -> Result<ChapterRecordDto, AppError>;
    async fn list_chapter_records(
        &self,
        user_id: String,
        query: ChapterRecordListQuery,
    ) -> Result<ChapterRecordListResponse, AppError>;
}
