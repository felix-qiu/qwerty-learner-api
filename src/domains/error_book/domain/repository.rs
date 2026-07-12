use async_trait::async_trait;
use sqlx::PgPool;

use crate::domains::error_book::domain::model::{ErrorWordGroup, ErrorWordRecord};

#[derive(Debug, Clone)]
pub struct ErrorWordFilter {
    pub dict: Option<String>,
    pub offset: i64,
    pub limit: i64,
    pub ascending: bool,
}

#[async_trait]
pub trait ErrorBookRepository: Send + Sync {
    async fn is_active_user(&self, pool: &PgPool, user_id: &str) -> Result<bool, sqlx::Error>;

    async fn is_published_dictionary(
        &self,
        pool: &PgPool,
        dict_id: &str,
    ) -> Result<bool, sqlx::Error>;

    async fn list_groups(
        &self,
        pool: &PgPool,
        user_id: &str,
        filter: &ErrorWordFilter,
    ) -> Result<(Vec<ErrorWordGroup>, i64), sqlx::Error>;

    async fn list_records(
        &self,
        pool: &PgPool,
        user_id: &str,
        groups: &[(String, String)],
    ) -> Result<Vec<ErrorWordRecord>, sqlx::Error>;
}
