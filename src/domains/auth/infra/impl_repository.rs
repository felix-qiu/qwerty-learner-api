use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};

use crate::domains::auth::domain::{
    model::{AuthUser, NewAuthUser, RefreshToken},
    repository::UserAuthRepository,
};

pub struct UserAuthRepo;

const AUTH_USER_COLUMNS: &str = r#"
    id,
    email,
    password_hash,
    display_name,
    avatar_url,
    role,
    status,
    last_login_at,
    created_at,
    updated_at
"#;

const REFRESH_TOKEN_COLUMNS: &str = r#"
    id,
    user_id,
    token_hash,
    user_agent,
    ip::text AS ip,
    expires_at,
    revoked_at,
    created_at
"#;

#[async_trait]
impl UserAuthRepository for UserAuthRepo {
    async fn create_user(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user: NewAuthUser,
    ) -> Result<AuthUser, sqlx::Error> {
        let auth_user = sqlx::query_as::<_, AuthUser>(&format!(
            r#"
            INSERT INTO users
                (id, email, password_hash, display_name, avatar_url, role, status)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7)
            RETURNING {AUTH_USER_COLUMNS}
            "#
        ))
        .bind(user.id)
        .bind(user.email)
        .bind(user.password_hash)
        .bind(user.display_name)
        .bind(user.avatar_url)
        .bind(user.role)
        .bind(user.status)
        .fetch_one(&mut **tx)
        .await?;

        Ok(auth_user)
    }

    async fn find_user_by_email(
        &self,
        pool: PgPool,
        email: &str,
    ) -> Result<Option<AuthUser>, sqlx::Error> {
        let auth_user = sqlx::query_as::<_, AuthUser>(&format!(
            r#"
            SELECT {AUTH_USER_COLUMNS}
              FROM users
             WHERE email_normalized = lower($1)
            "#
        ))
        .bind(email)
        .fetch_optional(&pool)
        .await?;

        Ok(auth_user)
    }

    async fn find_user_by_id(
        &self,
        pool: PgPool,
        id: &str,
    ) -> Result<Option<AuthUser>, sqlx::Error> {
        let auth_user = sqlx::query_as::<_, AuthUser>(&format!(
            r#"
            SELECT {AUTH_USER_COLUMNS}
              FROM users
             WHERE id = $1
            "#
        ))
        .bind(id)
        .fetch_optional(&pool)
        .await?;

        Ok(auth_user)
    }

    async fn find_user_by_id_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        id: &str,
    ) -> Result<Option<AuthUser>, sqlx::Error> {
        let auth_user = sqlx::query_as::<_, AuthUser>(&format!(
            r#"
            SELECT {AUTH_USER_COLUMNS}
              FROM users
             WHERE id = $1
            "#
        ))
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?;

        Ok(auth_user)
    }

    async fn update_last_login_at(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user_id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE users
               SET last_login_at = now()
             WHERE id = $1
            "#,
        )
        .bind(user_id)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    async fn create_refresh_token(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user_id: &str,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO refresh_tokens (user_id, token_hash, expires_at)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    async fn find_active_refresh_token(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        token_hash: &str,
    ) -> Result<Option<RefreshToken>, sqlx::Error> {
        let refresh_token = sqlx::query_as::<_, RefreshToken>(&format!(
            r#"
            SELECT {REFRESH_TOKEN_COLUMNS}
              FROM refresh_tokens
             WHERE token_hash = $1
               AND revoked_at IS NULL
               AND expires_at > now()
             FOR UPDATE
            "#
        ))
        .bind(token_hash)
        .fetch_optional(&mut **tx)
        .await?;

        Ok(refresh_token)
    }

    async fn revoke_refresh_token(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        token_hash: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE refresh_tokens
               SET revoked_at = now()
             WHERE token_hash = $1
               AND revoked_at IS NULL
            "#,
        )
        .bind(token_hash)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    async fn revoke_user_refresh_tokens(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user_id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE refresh_tokens
               SET revoked_at = now()
             WHERE user_id = $1
               AND revoked_at IS NULL
            "#,
        )
        .bind(user_id)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }
}
