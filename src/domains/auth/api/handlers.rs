use axum::{
    extract::{rejection::JsonRejection, Extension, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use validator::Validate;

use crate::{
    common::{app_state::AppState, error::AppError, jwt::Claims},
    domains::auth::dto::auth_dto::{LoginRequest, RefreshTokenRequest, RegisterRequest},
};

#[utoipa::path(
    post,
    path = "/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "Created", body = crate::domains::auth::dto::auth_dto::AuthResponse),
        (status = 409, description = "Conflict", body = crate::common::error::ErrorResponse)
    ),
    tag = "Auth"
)]
pub async fn register(
    State(state): State<AppState>,
    payload: Result<Json<RegisterRequest>, JsonRejection>,
) -> Result<impl IntoResponse, AppError> {
    let Json(payload) = payload.map_err(AppError::from)?;
    payload
        .validate()
        .map_err(|err| AppError::ValidationError(err.to_string()))?;

    let auth_response = state.auth_service.register(payload).await?;
    Ok((StatusCode::CREATED, Json(auth_response)))
}

#[utoipa::path(
    post,
    path = "/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "OK", body = crate::domains::auth::dto::auth_dto::AuthResponse),
        (status = 401, description = "Unauthorized", body = crate::common::error::ErrorResponse)
    ),
    tag = "Auth"
)]
pub async fn login(
    State(state): State<AppState>,
    payload: Result<Json<LoginRequest>, JsonRejection>,
) -> Result<impl IntoResponse, AppError> {
    let Json(payload) = payload.map_err(AppError::from)?;
    payload
        .validate()
        .map_err(|err| AppError::ValidationError(err.to_string()))?;

    let auth_response = state.auth_service.login(payload).await?;
    Ok(Json(auth_response))
}

#[utoipa::path(
    post,
    path = "/auth/refresh",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "OK", body = crate::domains::auth::dto::auth_dto::AuthResponse),
        (status = 401, description = "Unauthorized", body = crate::common::error::ErrorResponse)
    ),
    tag = "Auth"
)]
pub async fn refresh(
    State(state): State<AppState>,
    payload: Result<Json<RefreshTokenRequest>, JsonRejection>,
) -> Result<impl IntoResponse, AppError> {
    let Json(payload) = payload.map_err(AppError::from)?;
    payload
        .validate()
        .map_err(|err| AppError::ValidationError(err.to_string()))?;

    let auth_response = state.auth_service.refresh(payload).await?;
    Ok(Json(auth_response))
}

#[utoipa::path(
    get,
    path = "/auth/me",
    responses(
        (status = 200, description = "OK", body = crate::domains::auth::dto::auth_dto::AuthUserDto),
        (status = 401, description = "Unauthorized", body = crate::common::error::ErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tag = "Auth"
)]
pub async fn me(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, AppError> {
    let user = state.auth_service.me(claims.sub).await?;
    Ok(Json(user))
}

#[utoipa::path(
    post,
    path = "/auth/logout",
    responses(
        (status = 204, description = "No Content"),
        (status = 401, description = "Unauthorized", body = crate::common::error::ErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tag = "Auth"
)]
pub async fn logout(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, AppError> {
    state.auth_service.logout(claims.sub).await?;
    Ok(StatusCode::NO_CONTENT)
}
