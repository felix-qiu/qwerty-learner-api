use crate::{
    common::{app_state::AppState, dto::RestApiResponse, error::AppError},
    domains::user::dto::user_dto::{CreateUserDto, SearchUserDto, UpdateUserDto, UserDto},
};

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use validator::Validate;

#[utoipa::path(
    get,
    path = "/user/{id}",
    responses((status = 200, description = "Get user by ID", body = UserDto)),
    tag = "Users"
)]
pub async fn get_user_by_id(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let user = state.user_service.get_user_by_id(id).await?;
    Ok(RestApiResponse::success(user))
}

#[utoipa::path(
    post,
    path = "/user/list",
    request_body = SearchUserDto,
    responses((status = 200, description = "List users by condition", body = [UserDto])),
    tag = "Users"
)]
pub async fn get_user_list(
    State(state): State<AppState>,
    Json(payload): Json<SearchUserDto>,
) -> Result<impl IntoResponse, AppError> {
    let users = state.user_service.get_user_list(payload).await?;
    Ok(RestApiResponse::success(users))
}

#[utoipa::path(
    get,
    path = "/user",
    responses((status = 200, description = "List all users", body = [UserDto])),
    tag = "Users"
)]
pub async fn get_users(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let users = state.user_service.get_users().await?;
    Ok(RestApiResponse::success(users))
}

#[utoipa::path(
    post,
    path = "/user",
    request_body = CreateUserDto,
    responses((status = 200, description = "Create a new user", body = UserDto)),
    tag = "Users"
)]
pub async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserDto>,
) -> Result<impl IntoResponse, AppError> {
    payload.validate().map_err(|err| {
        tracing::error!("Validation error: {err}");
        AppError::ValidationError(format!("Invalid input: {}", err))
    })?;

    let user = state.user_service.create_user(payload).await?;
    Ok(RestApiResponse::success(user))
}

#[utoipa::path(
    put,
    path = "/user/{id}",
    request_body = UpdateUserDto,
    responses((status = 200, description = "Update user", body = UserDto)),
    tag = "Users"
)]
pub async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateUserDto>,
) -> Result<impl IntoResponse, AppError> {
    payload.validate().map_err(|err| {
        tracing::error!("Validation error: {err}");
        AppError::ValidationError(format!("Invalid input: {}", err))
    })?;

    let user = state.user_service.update_user(id, payload).await?;
    Ok(RestApiResponse::success(user))
}

#[utoipa::path(
    delete,
    path = "/user/{id}",
    responses((status = 200, description = "User deleted")),
    tag = "Users"
)]
pub async fn delete_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let message = state.user_service.delete_user(id).await?;
    Ok(RestApiResponse::success_with_message(message, ()))
}
