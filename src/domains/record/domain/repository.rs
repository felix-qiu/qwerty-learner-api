use async_trait::async_trait;
use sqlx::{PgPool, Postgres, Transaction};

use crate::domains::record::domain::model::{
    ChapterRecord, NewChapterRecord, NewWordRecord, WordRecord,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ChapterFilter {
    #[default]
    Any,
    Null,
    Value(i32),
}

#[derive(Debug, Clone, Default)]
pub struct WordRecordFilter {
    pub dict: Option<String>,
    pub chapter: ChapterFilter,
    pub word: Option<String>,
    pub wrong_only: bool,
    pub from: Option<i64>,
    pub to: Option<i64>,
    pub offset: i64,
    pub limit: i64,
    pub ascending: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ChapterRecordFilter {
    pub dict: Option<String>,
    pub chapter: ChapterFilter,
    pub from: Option<i64>,
    pub to: Option<i64>,
    pub offset: i64,
    pub limit: i64,
}

#[async_trait]
pub trait RecordRepository: Send + Sync {
    async fn is_active_user(&self, pool: &PgPool, user_id: &str) -> Result<bool, sqlx::Error>;

    async fn create_word_record(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        record: NewWordRecord,
    ) -> Result<WordRecord, sqlx::Error>;

    async fn list_word_records(
        &self,
        pool: &PgPool,
        user_id: &str,
        filter: &WordRecordFilter,
    ) -> Result<(Vec<WordRecord>, i64), sqlx::Error>;

    async fn delete_word_records(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user_id: &str,
        word: &str,
        dict: &str,
    ) -> Result<u64, sqlx::Error>;

    async fn create_chapter_record(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        record: NewChapterRecord,
    ) -> Result<ChapterRecord, sqlx::Error>;

    async fn list_chapter_records(
        &self,
        pool: &PgPool,
        user_id: &str,
        filter: &ChapterRecordFilter,
    ) -> Result<(Vec<ChapterRecord>, i64), sqlx::Error>;
}
