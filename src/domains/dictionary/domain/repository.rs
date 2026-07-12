use async_trait::async_trait;
use sqlx::{PgPool, Postgres, Transaction};

use crate::domains::dictionary::{
    domain::model::{Dictionary, NewDictionary, NewWord, Word},
    dto::dictionary_dto::DictionaryListQuery,
};

#[async_trait]
pub trait DictionaryRepository: Send + Sync {
    async fn list(
        &self,
        pool: &PgPool,
        query: &DictionaryListQuery,
    ) -> Result<(Vec<Dictionary>, i64), sqlx::Error>;

    async fn find_published_by_id(
        &self,
        pool: &PgPool,
        id: &str,
    ) -> Result<Option<Dictionary>, sqlx::Error>;

    async fn find_by_id(&self, pool: &PgPool, id: &str) -> Result<Option<Dictionary>, sqlx::Error>;

    async fn is_active_admin(&self, pool: &PgPool, user_id: &str) -> Result<bool, sqlx::Error>;

    async fn create(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        dictionary: NewDictionary,
    ) -> Result<Dictionary, sqlx::Error>;

    async fn update(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        id: &str,
        dictionary: NewDictionary,
    ) -> Result<Option<Dictionary>, sqlx::Error>;

    async fn delete(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        id: &str,
    ) -> Result<bool, sqlx::Error>;

    async fn list_words(
        &self,
        pool: &PgPool,
        dict_id: &str,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<Word>, sqlx::Error>;

    async fn find_word(
        &self,
        pool: &PgPool,
        dict_id: &str,
        word_name: &str,
    ) -> Result<Option<Word>, sqlx::Error>;

    async fn replace_words(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        dict_id: &str,
        words: Vec<NewWord>,
    ) -> Result<(i64, Dictionary), sqlx::Error>;

    async fn append_words(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        dict_id: &str,
        words: Vec<NewWord>,
    ) -> Result<(i64, Dictionary), sqlx::Error>;
}
