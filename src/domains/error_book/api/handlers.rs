use axum::{
    extract::{rejection::QueryRejection, Extension, Path, Query, State},
    response::IntoResponse,
    Json,
};

use crate::{
    common::{app_state::AppState, error::AppError, jwt::Claims},
    domains::error_book::dto::error_book_dto::ErrorBookListQuery,
};

#[utoipa::path(
    get,
    path = "/error-book",
    operation_id = "listErrorBook",
    summary = "全局错题本",
    params(ErrorBookListQuery),
    responses((status = 200, description = "OK", body = crate::domains::error_book::dto::error_book_dto::ErrorBookListResponse)),
    security(("bearerAuth" = [])),
    tag = "ErrorBook"
)]
pub async fn list_error_book(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    query: Result<Query<ErrorBookListQuery>, QueryRejection>,
) -> Result<impl IntoResponse, AppError> {
    let Query(query) = query.map_err(AppError::from)?;
    Ok(Json(
        state
            .error_book_service
            .list_error_book(claims.sub, query)
            .await?,
    ))
}

#[utoipa::path(
    get,
    path = "/dictionaries/{dictId}/error-words",
    operation_id = "getDictErrorWords",
    summary = "某词典错题聚合",
    params(("dictId" = String, Path, example = "cet4")),
    responses(
        (status = 200, description = "OK", body = crate::domains::error_book::dto::error_book_dto::DictErrorWordsResponse),
        (status = 404, description = "Not Found", body = crate::common::error::ErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tag = "ErrorBook"
)]
pub async fn get_dictionary_error_words(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(dict_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(
        state
            .error_book_service
            .get_dictionary_error_words(claims.sub, dict_id)
            .await?,
    ))
}
