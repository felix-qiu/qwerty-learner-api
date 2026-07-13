use axum::{
    extract::{rejection::JsonRejection, Extension, State},
    response::IntoResponse,
    Json,
};
use serde_json::Value;

use crate::{
    common::{app_state::AppState, error::AppError, jwt::Claims},
    domains::settings::dto::settings_dto::UserSettingsDto,
};

#[utoipa::path(
    get,
    path = "/settings",
    operation_id = "getSettings",
    summary = "获取用户设置",
    responses((status = 200, description = "OK", body = crate::domains::settings::dto::settings_dto::UserSettingsDto)),
    security(("bearerAuth" = [])),
    tag = "Settings"
)]
pub async fn get_settings(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(state.settings_service.get_settings(claims.sub).await?))
}

#[utoipa::path(
    put,
    path = "/settings",
    operation_id = "putSettings",
    summary = "全量更新设置",
    request_body = UserSettingsDto,
    responses((status = 200, description = "OK", body = crate::domains::settings::dto::settings_dto::UserSettingsDto)),
    security(("bearerAuth" = [])),
    tag = "Settings"
)]
pub async fn put_settings(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    payload: Result<Json<UserSettingsDto>, JsonRejection>,
) -> Result<impl IntoResponse, AppError> {
    let Json(payload) = payload.map_err(settings_json_error)?;
    Ok(Json(
        state
            .settings_service
            .put_settings(claims.sub, payload)
            .await?,
    ))
}

#[utoipa::path(
    patch,
    path = "/settings",
    operation_id = "patchSettings",
    summary = "部分更新设置",
    request_body(content = crate::domains::settings::dto::settings_dto::UserSettingsPatchDto, description = "任意 UserSettings 子集，对象字段深合并"),
    responses((status = 200, description = "OK", body = crate::domains::settings::dto::settings_dto::UserSettingsDto)),
    security(("bearerAuth" = [])),
    tag = "Settings"
)]
pub async fn patch_settings(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<impl IntoResponse, AppError> {
    let Json(payload) = payload.map_err(AppError::from)?;
    Ok(Json(
        state
            .settings_service
            .patch_settings(claims.sub, payload)
            .await?,
    ))
}

fn settings_json_error(rejection: JsonRejection) -> AppError {
    match rejection {
        JsonRejection::JsonDataError(error) => AppError::ValidationError(error.body_text()),
        rejection => AppError::from(rejection),
    }
}
