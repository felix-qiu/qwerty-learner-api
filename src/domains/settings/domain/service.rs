use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use sqlx::PgPool;

use crate::{common::error::AppError, domains::settings::dto::settings_dto::UserSettingsDto};

#[async_trait]
pub trait SettingsServiceTrait: Send + Sync {
    fn create_service(pool: PgPool) -> Arc<dyn SettingsServiceTrait>
    where
        Self: Sized;

    async fn get_settings(&self, user_id: String) -> Result<UserSettingsDto, AppError>;

    async fn put_settings(
        &self,
        user_id: String,
        settings: UserSettingsDto,
    ) -> Result<UserSettingsDto, AppError>;

    async fn patch_settings(
        &self,
        user_id: String,
        patch: Value,
    ) -> Result<UserSettingsDto, AppError>;
}
