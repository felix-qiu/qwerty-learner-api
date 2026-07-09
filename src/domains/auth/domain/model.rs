//! Authentication domain models.

use chrono::{DateTime, Utc};
use sqlx::prelude::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct AuthUser {
    pub id: String,
    pub email: String,
    pub password_hash: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub role: String,
    pub status: String,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewAuthUser {
    pub id: String,
    pub email: String,
    pub password_hash: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub role: String,
    pub status: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct RefreshToken {
    pub id: i64,
    pub user_id: String,
    pub token_hash: String,
    pub user_agent: Option<String>,
    pub ip: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct UserToken {
    pub id: i64,
    pub user_id: String,
    pub purpose: String,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
