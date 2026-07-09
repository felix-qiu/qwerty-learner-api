//! Repository contract for authentication data.

use super::model::{AuthUser, NewAuthUser, RefreshToken};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};

#[async_trait]
pub trait UserAuthRepository: Send + Sync {
    async fn create_user(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user: NewAuthUser,
    ) -> Result<AuthUser, sqlx::Error>;

    async fn find_user_by_email(
        &self,
        pool: PgPool,
        email: &str,
    ) -> Result<Option<AuthUser>, sqlx::Error>;

    async fn find_user_by_id(
        &self,
        pool: PgPool,
        id: &str,
    ) -> Result<Option<AuthUser>, sqlx::Error>;

    async fn find_user_by_id_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        id: &str,
    ) -> Result<Option<AuthUser>, sqlx::Error>;

    async fn update_last_login_at(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user_id: &str,
    ) -> Result<(), sqlx::Error>;

    async fn create_refresh_token(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user_id: &str,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error>;

    async fn find_active_refresh_token(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        token_hash: &str,
    ) -> Result<Option<RefreshToken>, sqlx::Error>;

    async fn revoke_refresh_token(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        token_hash: &str,
    ) -> Result<(), sqlx::Error>;

    async fn revoke_user_refresh_tokens(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user_id: &str,
    ) -> Result<(), sqlx::Error>;
}
