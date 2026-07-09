use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};

use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::LazyLock;
use std::{env, fmt::Display};
use uuid::Uuid;

use super::error::AppError;

pub const ACCESS_TOKEN_EXPIRES_IN_SECONDS: i64 = 3600;
pub const REFRESH_TOKEN_EXPIRES_IN_SECONDS: i64 = 60 * 60 * 24 * 30;

pub static KEYS: LazyLock<Keys> = LazyLock::new(|| {
    dotenvy::dotenv().ok();

    let secret = env::var("JWT_SECRET_KEY").expect("JWT_SECRET_KEY must be set");
    Keys::new(secret.as_bytes())
});

pub struct Keys {
    pub encoding: EncodingKey,
    pub decoding: DecodingKey,
}

impl Keys {
    fn new(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TokenKind {
    Access,
    Refresh,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub jti: String,
    pub token_type: TokenKind,
}

impl Display for Claims {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "user_id: {}", self.sub)
    }
}

pub fn make_access_token(user_id: &str) -> Result<String, AppError> {
    let (token, _) = make_token(
        user_id,
        TokenKind::Access,
        Duration::seconds(ACCESS_TOKEN_EXPIRES_IN_SECONDS),
    )?;
    Ok(token)
}

pub fn make_refresh_token(user_id: &str) -> Result<(String, DateTime<Utc>), AppError> {
    make_token(
        user_id,
        TokenKind::Refresh,
        Duration::seconds(REFRESH_TOKEN_EXPIRES_IN_SECONDS),
    )
}

pub fn decode_refresh_token(token: &str) -> Result<Claims, AppError> {
    let claims = decode_claims(token)?;
    if claims.token_type != TokenKind::Refresh {
        return Err(AppError::InvalidToken);
    }
    Ok(claims)
}

pub fn token_hash(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    hex::encode(digest)
}

fn make_token(
    user_id: &str,
    token_type: TokenKind,
    lifetime: Duration,
) -> Result<(String, DateTime<Utc>), AppError> {
    let now = Utc::now();
    let expires_at = now + lifetime;
    let claims = Claims {
        sub: user_id.to_string(),
        exp: expires_at.timestamp() as usize,
        iat: now.timestamp() as usize,
        jti: Uuid::new_v4().to_string(),
        token_type,
    };

    let token =
        encode(&Header::default(), &claims, &KEYS.encoding).map_err(|_| AppError::TokenCreation)?;

    Ok((token, expires_at))
}

fn decode_claims(token: &str) -> Result<Claims, AppError> {
    decode::<Claims>(token, &KEYS.decoding, &Validation::default())
        .map(|token_data| token_data.claims)
        .map_err(|err| {
            tracing::error!("Error decoding token: {:?}", err);
            AppError::InvalidToken
        })
}

pub async fn jwt_auth<B>(mut req: Request<B>, next: Next) -> Result<Response, Response>
where
    B: Send + Into<axum::body::Body>,
{
    let token = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|header| header.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| AppError::InvalidToken.into_response())?;

    let claims = decode_claims(token).map_err(|err| err.into_response())?;
    if claims.token_type != TokenKind::Access {
        return Err(AppError::InvalidToken.into_response());
    }

    req.extensions_mut().insert(claims);
    Ok(next.run(req.map(Into::into)).await)
}
