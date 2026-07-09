use axum::http::{Method, StatusCode};

use clean_axum_demo::{
    common::{dto::RestApiResponse, error::ErrorResponse},
    domains::user::dto::user_dto::{CreateUserDto, SearchUserDto, UpdateUserDto, UserDto},
};

mod test_helpers;

use test_helpers::{deserialize_json_body, request_with_auth, request_with_auth_and_body};

async fn create_user() -> (CreateUserDto, UserDto) {
    let unique = uuid::Uuid::new_v4();
    let payload = CreateUserDto {
        email: format!("user-{unique}@example.com"),
        password: "test_password".to_string(),
        display_name: Some(format!("Test User {unique}")),
        avatar_url: Some("https://example.com/avatar.png".to_string()),
        role: Some("user".to_string()),
        status: Some("active".to_string()),
    };

    let response = request_with_auth_and_body(Method::POST, "/user", &payload);
    let (parts, body) = response.await.into_parts();

    assert_eq!(parts.status, StatusCode::OK);

    let response_body: RestApiResponse<UserDto> = deserialize_json_body(body).await.unwrap();
    assert_eq!(response_body.0.status, StatusCode::OK.as_u16());

    let user = response_body.0.data.unwrap();
    (payload, user)
}

#[tokio::test]
async fn test_create_user() {
    let (payload, user) = create_user().await;

    assert!(user.id.starts_with("usr_"));
    assert_eq!(user.email, payload.email);
    assert_eq!(user.display_name, payload.display_name);
    assert_eq!(user.avatar_url, payload.avatar_url);
    assert_eq!(user.role, "user");
    assert_eq!(user.status, "active");
    assert!(user.last_login_at.is_none());
}

#[tokio::test]
async fn test_get_users() {
    let response = request_with_auth(Method::GET, "/user");
    let (parts, body) = response.await.into_parts();

    assert_eq!(parts.status, StatusCode::OK);

    let response_body: RestApiResponse<Vec<UserDto>> = deserialize_json_body(body).await.unwrap();
    assert_eq!(response_body.0.status, StatusCode::OK.as_u16());
    assert!(!response_body.0.data.unwrap().is_empty());
}

#[tokio::test]
async fn test_get_user_list() {
    let (_, created_user) = create_user().await;
    let payload = SearchUserDto {
        id: None,
        email: Some(created_user.email.clone()),
        display_name: None,
        role: Some("user".to_string()),
        status: Some("active".to_string()),
    };

    let response = request_with_auth_and_body(Method::POST, "/user/list", &payload);
    let (parts, body) = response.await.into_parts();

    assert_eq!(parts.status, StatusCode::OK);

    let response_body: RestApiResponse<Vec<UserDto>> = deserialize_json_body(body).await.unwrap();
    assert_eq!(response_body.0.status, StatusCode::OK.as_u16());

    let users = response_body.0.data.unwrap();
    assert!(users.iter().any(|user| user.id == created_user.id));
}

#[tokio::test]
async fn test_get_user_by_id() {
    let (_, created_user) = create_user().await;

    let url = format!("/user/{}", created_user.id);
    let response = request_with_auth(Method::GET, url.as_str());
    let (parts, body) = response.await.into_parts();

    assert_eq!(parts.status, StatusCode::OK);

    let response_body: RestApiResponse<UserDto> = deserialize_json_body(body).await.unwrap();
    assert_eq!(response_body.0.status, StatusCode::OK.as_u16());

    let user = response_body.0.data.unwrap();
    assert_eq!(user.id, created_user.id);
    assert_eq!(user.email, created_user.email);
    assert_eq!(user.display_name, created_user.display_name);
    assert_eq!(user.avatar_url, created_user.avatar_url);
    assert_eq!(user.role, created_user.role);
    assert_eq!(user.status, created_user.status);
}

#[tokio::test]
async fn test_update_user() {
    let (_, created_user) = create_user().await;
    let unique = uuid::Uuid::new_v4();
    let payload = UpdateUserDto {
        email: Some(format!("updated-{unique}@example.com")),
        password: Some("updated_password".to_string()),
        display_name: Some("Updated User".to_string()),
        avatar_url: Some("https://example.com/updated-avatar.png".to_string()),
        role: Some("admin".to_string()),
        status: Some("disabled".to_string()),
    };

    let url = format!("/user/{}", created_user.id);
    let response = request_with_auth_and_body(Method::PUT, url.as_str(), &payload);
    let (parts, body) = response.await.into_parts();

    assert_eq!(parts.status, StatusCode::OK);

    let response_body: RestApiResponse<UserDto> = deserialize_json_body(body).await.unwrap();
    assert_eq!(response_body.0.status, StatusCode::OK.as_u16());

    let user = response_body.0.data.unwrap();
    assert_eq!(user.id, created_user.id);
    assert_eq!(Some(user.email), payload.email);
    assert_eq!(user.display_name, payload.display_name);
    assert_eq!(user.avatar_url, payload.avatar_url);
    assert_eq!(Some(user.role), payload.role);
    assert_eq!(Some(user.status), payload.status);
}

#[tokio::test]
async fn test_delete_user_not_found() {
    let non_existent_id = format!("usr_{}", uuid::Uuid::new_v4());

    let url = format!("/user/{}", non_existent_id);
    let response = request_with_auth(Method::DELETE, url.as_str());
    let (parts, body) = response.await.into_parts();

    assert_eq!(parts.status, StatusCode::NOT_FOUND);

    let error: ErrorResponse = deserialize_json_body(body).await.unwrap();
    assert_eq!(error.code, "NOT_FOUND");
}

#[tokio::test]
async fn test_delete_user() {
    let (_, user) = create_user().await;
    let url = format!("/user/{}", user.id);
    let response = request_with_auth(Method::DELETE, url.as_str());
    let (parts, body) = response.await.into_parts();

    assert_eq!(parts.status, StatusCode::OK);

    let response_body: RestApiResponse<()> = deserialize_json_body(body).await.unwrap();
    assert_eq!(response_body.0.status, StatusCode::OK.as_u16());
}
