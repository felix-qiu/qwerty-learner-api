use async_trait::async_trait;
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};

#[async_trait]
pub trait SettingsRepository: Send + Sync {
    async fn is_active_user(&self, pool: &PgPool, user_id: &str) -> Result<bool, sqlx::Error>;

    async fn get_or_create(&self, pool: &PgPool, user_id: &str) -> Result<Value, sqlx::Error>;

    async fn replace(
        &self,
        pool: &PgPool,
        user_id: &str,
        settings: Value,
    ) -> Result<Value, sqlx::Error>;

    async fn get_for_update(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user_id: &str,
    ) -> Result<Value, sqlx::Error>;

    async fn update(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user_id: &str,
        settings: Value,
    ) -> Result<Value, sqlx::Error>;
}
