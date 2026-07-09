//! Authentication service contract.

use std::sync::Arc;

use sqlx::PgPool;

use crate::{
    common::error::AppError,
    domains::auth::dto::auth_dto::{
        AuthResponse, AuthUserDto, LoginRequest, RefreshTokenRequest, RegisterRequest,
    },
};

#[async_trait::async_trait]
/// Trait defining the contract for authentication-related operations.
/// Implementors are responsible for handling user creation and login logic.
pub trait AuthServiceTrait: Send + Sync {
    /// constructor for the service.
    fn create_service(pool: PgPool) -> Arc<dyn AuthServiceTrait>
    where
        Self: Sized;

    async fn register(&self, payload: RegisterRequest) -> Result<AuthResponse, AppError>;

    async fn login(&self, payload: LoginRequest) -> Result<AuthResponse, AppError>;

    async fn refresh(&self, payload: RefreshTokenRequest) -> Result<AuthResponse, AppError>;

    async fn me(&self, user_id: String) -> Result<AuthUserDto, AppError>;

    async fn logout(&self, user_id: String) -> Result<(), AppError>;
}
