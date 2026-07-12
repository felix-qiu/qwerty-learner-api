use std::sync::Arc;

use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    common::{
        error::AppError,
        hash_util,
        jwt::{
            decode_refresh_token, make_access_token, make_refresh_token, token_hash,
            ACCESS_TOKEN_EXPIRES_IN_SECONDS,
        },
    },
    domains::auth::{
        domain::{
            model::{AuthUser, NewAuthUser},
            repository::UserAuthRepository,
            service::AuthServiceTrait,
        },
        dto::auth_dto::{
            AuthResponse, AuthUserDto, LoginRequest, RefreshTokenRequest, RegisterRequest,
        },
        infra::impl_repository::UserAuthRepo,
    },
};

#[derive(Clone)]
pub struct AuthService {
    pool: PgPool,
    repo: Arc<dyn UserAuthRepository + Send + Sync>,
}

#[async_trait::async_trait]
impl AuthServiceTrait for AuthService {
    fn create_service(pool: PgPool) -> Arc<dyn AuthServiceTrait> {
        Arc::new(Self {
            pool,
            repo: Arc::new(UserAuthRepo {}),
        })
    }

    async fn register(&self, payload: RegisterRequest) -> Result<AuthResponse, AppError> {
        let email = normalize_email(&payload.email)?;
        if self
            .repo
            .find_user_by_email(self.pool.clone(), &email)
            .await
            .map_err(AppError::DatabaseError)?
            .is_some()
        {
            return Err(AppError::Conflict("Email already registered".into()));
        }

        let password_hash =
            hash_util::hash_password(&payload.password).map_err(|_| AppError::InternalError)?;

        let mut tx = self.pool.begin().await?;
        let user = self
            .repo
            .create_user(
                &mut tx,
                NewAuthUser {
                    id: format!("usr_{}", Uuid::new_v4()),
                    email,
                    password_hash,
                    display_name: payload.display_name,
                    avatar_url: None,
                    role: "user".to_string(),
                    status: "active".to_string(),
                },
            )
            .await
            .map_err(map_registration_error)?;

        let response = self.create_auth_response(&mut tx, user).await?;
        tx.commit().await?;

        Ok(response)
    }

    async fn login(&self, payload: LoginRequest) -> Result<AuthResponse, AppError> {
        let email = normalize_email(&payload.email)?;
        let user = self
            .repo
            .find_user_by_email(self.pool.clone(), &email)
            .await
            .map_err(AppError::DatabaseError)?
            .ok_or(AppError::WrongCredentials)?;

        ensure_login_user(&user)?;

        if !hash_util::verify_password(&user.password_hash, &payload.password) {
            return Err(AppError::WrongCredentials);
        }

        let mut tx = self.pool.begin().await?;
        self.repo
            .update_last_login_at(&mut tx, &user.id)
            .await
            .map_err(AppError::DatabaseError)?;

        let response = self.create_auth_response(&mut tx, user).await?;
        tx.commit().await?;

        Ok(response)
    }

    async fn refresh(&self, payload: RefreshTokenRequest) -> Result<AuthResponse, AppError> {
        let refresh_claims = decode_refresh_token(&payload.refresh_token)?;
        let refresh_hash = token_hash(&payload.refresh_token);

        let mut tx = self.pool.begin().await?;
        let stored_token = self
            .repo
            .find_active_refresh_token(&mut tx, &refresh_hash)
            .await
            .map_err(AppError::DatabaseError)?
            .ok_or(AppError::InvalidToken)?;

        if stored_token.user_id != refresh_claims.sub {
            return Err(AppError::InvalidToken);
        }

        self.repo
            .revoke_refresh_token(&mut tx, &refresh_hash)
            .await
            .map_err(AppError::DatabaseError)?;

        let user = self
            .repo
            .find_user_by_id_in_tx(&mut tx, &stored_token.user_id)
            .await
            .map_err(AppError::DatabaseError)?
            .ok_or(AppError::InvalidToken)?;

        ensure_token_user(&user)?;

        let response = self.create_auth_response(&mut tx, user).await?;
        tx.commit().await?;

        Ok(response)
    }

    async fn me(&self, user_id: String) -> Result<AuthUserDto, AppError> {
        let user = self
            .repo
            .find_user_by_id(self.pool.clone(), &user_id)
            .await
            .map_err(AppError::DatabaseError)?
            .ok_or(AppError::InvalidToken)?;

        ensure_token_user(&user)?;

        Ok(AuthUserDto::from(user))
    }

    async fn logout(&self, user_id: String) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;
        self.repo
            .revoke_user_refresh_tokens(&mut tx, &user_id)
            .await
            .map_err(AppError::DatabaseError)?;
        tx.commit().await?;

        Ok(())
    }
}

impl AuthService {
    async fn create_auth_response(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        user: AuthUser,
    ) -> Result<AuthResponse, AppError> {
        let access_token = make_access_token(&user.id)?;
        let (refresh_token, refresh_expires_at) = make_refresh_token(&user.id)?;
        let refresh_hash = token_hash(&refresh_token);

        self.repo
            .create_refresh_token(tx, &user.id, &refresh_hash, refresh_expires_at)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(AuthResponse {
            user: AuthUserDto::from(user),
            access_token,
            refresh_token,
            expires_in: ACCESS_TOKEN_EXPIRES_IN_SECONDS,
        })
    }
}

impl From<AuthUser> for AuthUserDto {
    fn from(user: AuthUser) -> Self {
        Self {
            id: user.id,
            email: user.email,
            display_name: user.display_name,
            created_at: user.created_at.timestamp(),
        }
    }
}

fn normalize_email(email: &str) -> Result<String, AppError> {
    let email = email.trim().to_ascii_lowercase();
    if email.is_empty() {
        return Err(AppError::ValidationError("Email is required".into()));
    }
    Ok(email)
}

fn ensure_login_user(user: &AuthUser) -> Result<(), AppError> {
    if user.status == "active" {
        return Ok(());
    }
    Err(AppError::WrongCredentials)
}

fn ensure_token_user(user: &AuthUser) -> Result<(), AppError> {
    if user.status == "active" {
        return Ok(());
    }
    Err(AppError::InvalidToken)
}

fn map_registration_error(error: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(database_error) = &error {
        if database_error.is_unique_violation() {
            return AppError::Conflict("Email already registered".into());
        }
    }
    AppError::DatabaseError(error)
}
