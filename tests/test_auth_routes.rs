use axum::http::{Method, StatusCode};

use clean_axum_demo::{
    common::error::ErrorResponse,
    domains::auth::dto::auth_dto::{
        AuthResponse, AuthUserDto, LoginRequest, RefreshTokenRequest, RegisterRequest,
    },
};
use test_helpers::{
    deserialize_json_body, request_with_bearer_token, request_with_body, TEST_AUTH_EMAIL,
    TEST_AUTH_PASSWORD,
};

mod test_helpers;

#[tokio::test]
async fn test_register_user() {
    let unique = uuid::Uuid::new_v4();
    let payload = RegisterRequest {
        email: format!("register-{unique}@example.com"),
        password: "test_password".to_string(),
        display_name: Some("Register User".to_string()),
    };

    let response = request_with_body(Method::POST, "/api/v1/auth/register", &payload);
    let (parts, body) = response.await.into_parts();

    assert_eq!(parts.status, StatusCode::CREATED);

    let auth_response: AuthResponse = deserialize_json_body(body).await.unwrap();
    assert_eq!(auth_response.user.email, payload.email);
    assert_eq!(auth_response.user.display_name, payload.display_name);
    assert!(!auth_response.access_token.is_empty());
    assert!(!auth_response.refresh_token.is_empty());
    assert_eq!(auth_response.expires_in, 3600);
}

#[tokio::test]
async fn test_register_duplicate_email() {
    let payload = RegisterRequest {
        email: TEST_AUTH_EMAIL.to_string(),
        password: TEST_AUTH_PASSWORD.to_string(),
        display_name: Some("Duplicate User".to_string()),
    };

    let response = request_with_body(Method::POST, "/api/v1/auth/register", &payload);
    let (parts, body) = response.await.into_parts();

    assert_eq!(parts.status, StatusCode::CONFLICT);

    let error: ErrorResponse = deserialize_json_body(body).await.unwrap();
    assert_eq!(error.code, "CONFLICT");
}

#[tokio::test]
async fn test_login_user() {
    let payload = LoginRequest {
        email: TEST_AUTH_EMAIL.to_string(),
        password: TEST_AUTH_PASSWORD.to_string(),
    };

    let response = request_with_body(Method::POST, "/api/v1/auth/login", &payload);
    let (parts, body) = response.await.into_parts();

    assert_eq!(parts.status, StatusCode::OK);

    let auth_response: AuthResponse = deserialize_json_body(body).await.unwrap();
    assert_eq!(auth_response.user.email, TEST_AUTH_EMAIL);
    assert!(!auth_response.access_token.is_empty());
    assert!(!auth_response.refresh_token.is_empty());
    assert_eq!(auth_response.expires_in, 3600);
}

#[tokio::test]
async fn test_login_user_fail() {
    let payload = LoginRequest {
        email: TEST_AUTH_EMAIL.to_string(),
        password: uuid::Uuid::new_v4().to_string(),
    };

    let response = request_with_body(Method::POST, "/api/v1/auth/login", &payload);
    let (parts, body) = response.await.into_parts();

    assert_eq!(parts.status, StatusCode::UNAUTHORIZED);

    let error: ErrorResponse = deserialize_json_body(body).await.unwrap();
    assert_eq!(error.code, "UNAUTHORIZED");
}

#[tokio::test]
async fn test_refresh_token() {
    let login = register_unique_user().await;
    let payload = RefreshTokenRequest {
        refresh_token: login.refresh_token.clone(),
    };

    let response = request_with_body(Method::POST, "/api/v1/auth/refresh", &payload);
    let (parts, body) = response.await.into_parts();

    assert_eq!(parts.status, StatusCode::OK);

    let auth_response: AuthResponse = deserialize_json_body(body).await.unwrap();
    assert_eq!(auth_response.user.email, login.user.email);
    assert_ne!(auth_response.refresh_token, login.refresh_token);
    assert!(!auth_response.access_token.is_empty());
}

#[tokio::test]
async fn test_get_me() {
    let login = register_unique_user().await;

    let response = request_with_bearer_token(Method::GET, "/api/v1/auth/me", &login.access_token);
    let (parts, body) = response.await.into_parts();

    assert_eq!(parts.status, StatusCode::OK);

    let user: AuthUserDto = deserialize_json_body(body).await.unwrap();
    assert_eq!(user.email, login.user.email);
}

#[tokio::test]
async fn test_logout_revokes_refresh_tokens() {
    let login = register_unique_user().await;

    let logout_response =
        request_with_bearer_token(Method::POST, "/api/v1/auth/logout", &login.access_token);
    let (logout_parts, _) = logout_response.await.into_parts();
    assert_eq!(logout_parts.status, StatusCode::NO_CONTENT);

    let payload = RefreshTokenRequest {
        refresh_token: login.refresh_token,
    };
    let refresh_response = request_with_body(Method::POST, "/api/v1/auth/refresh", &payload);
    let (refresh_parts, body) = refresh_response.await.into_parts();

    assert_eq!(refresh_parts.status, StatusCode::UNAUTHORIZED);

    let error: ErrorResponse = deserialize_json_body(body).await.unwrap();
    assert_eq!(error.code, "UNAUTHORIZED");
}

async fn register_unique_user() -> AuthResponse {
    let unique = uuid::Uuid::new_v4();
    let payload = RegisterRequest {
        email: format!("auth-{unique}@example.com"),
        password: "test_password".to_string(),
        display_name: Some("Auth Test User".to_string()),
    };

    let response = request_with_body(Method::POST, "/api/v1/auth/register", &payload);
    let (parts, body) = response.await.into_parts();
    assert_eq!(parts.status, StatusCode::CREATED);

    deserialize_json_body(body).await.unwrap()
}
