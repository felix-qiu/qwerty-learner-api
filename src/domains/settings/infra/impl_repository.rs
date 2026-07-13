use async_trait::async_trait;
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};

use crate::domains::settings::domain::repository::SettingsRepository;

pub struct SettingsRepo;

#[async_trait]
impl SettingsRepository for SettingsRepo {
    async fn is_active_user(&self, pool: &PgPool, user_id: &str) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM users WHERE id = $1 AND status = 'active')",
        )
        .bind(user_id)
        .fetch_one(pool)
        .await
    }

    async fn get_or_create(&self, pool: &PgPool, user_id: &str) -> Result<Value, sqlx::Error> {
        sqlx::query("INSERT INTO user_settings (user_id) VALUES ($1) ON CONFLICT DO NOTHING")
            .bind(user_id)
            .execute(pool)
            .await?;
        sqlx::query_scalar::<_, Value>("SELECT settings FROM user_settings WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(pool)
            .await
    }

    async fn replace(
        &self,
        pool: &PgPool,
        user_id: &str,
        settings: Value,
    ) -> Result<Value, sqlx::Error> {
        sqlx::query_scalar::<_, Value>(
            r#"
            INSERT INTO user_settings (user_id, settings)
            VALUES ($1, $2)
            ON CONFLICT (user_id) DO UPDATE SET settings = EXCLUDED.settings
            RETURNING settings
            "#,
        )
        .bind(user_id)
        .bind(settings)
        .fetch_one(pool)
        .await
    }

    async fn get_for_update(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user_id: &str,
    ) -> Result<Value, sqlx::Error> {
        sqlx::query("INSERT INTO user_settings (user_id) VALUES ($1) ON CONFLICT DO NOTHING")
            .bind(user_id)
            .execute(&mut **tx)
            .await?;
        sqlx::query_scalar::<_, Value>(
            "SELECT settings FROM user_settings WHERE user_id = $1 FOR UPDATE",
        )
        .bind(user_id)
        .fetch_one(&mut **tx)
        .await
    }

    async fn update(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user_id: &str,
        settings: Value,
    ) -> Result<Value, sqlx::Error> {
        sqlx::query_scalar::<_, Value>(
            "UPDATE user_settings SET settings = $2 WHERE user_id = $1 RETURNING settings",
        )
        .bind(user_id)
        .bind(settings)
        .fetch_one(&mut **tx)
        .await
    }
}
