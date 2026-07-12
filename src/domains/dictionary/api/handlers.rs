use axum::{
    extract::{
        rejection::{JsonRejection, QueryRejection},
        Extension, Path, Query, State,
    },
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::{
    common::{app_state::AppState, error::AppError, jwt::Claims},
    domains::dictionary::dto::dictionary_dto::{
        DictionaryCreateDto, DictionaryListQuery, WordBulkRequest, WordListQuery,
    },
};

#[utoipa::path(
    get,
    path = "/dictionaries",
    params(DictionaryListQuery),
    responses((status = 200, body = crate::domains::dictionary::dto::dictionary_dto::DictionaryListResponse)),
    tag = "Dictionaries"
)]
pub async fn list_dictionaries(
    State(state): State<AppState>,
    query: Result<Query<DictionaryListQuery>, QueryRejection>,
) -> Result<impl IntoResponse, AppError> {
    let Query(query) = query.map_err(AppError::from)?;
    Ok(Json(state.dictionary_service.list(query).await?))
}

#[utoipa::path(
    get,
    path = "/dictionaries/{dictId}",
    params(("dictId" = String, Path)),
    responses(
        (status = 200, body = crate::domains::dictionary::dto::dictionary_dto::DictionaryDto),
        (status = 404, body = crate::common::error::ErrorResponse)
    ),
    tag = "Dictionaries"
)]
pub async fn get_dictionary(
    State(state): State<AppState>,
    Path(dict_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(state.dictionary_service.get(dict_id).await?))
}

#[utoipa::path(
    get,
    path = "/dictionaries/{dictId}/words",
    params(("dictId" = String, Path), WordListQuery),
    responses(
        (status = 200, body = crate::domains::dictionary::dto::dictionary_dto::WordListResponse),
        (status = 404, body = crate::common::error::ErrorResponse)
    ),
    tag = "Dictionaries"
)]
pub async fn get_dictionary_words(
    State(state): State<AppState>,
    Path(dict_id): Path<String>,
    Query(query): Query<WordListQuery>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(
        state.dictionary_service.get_words(dict_id, query).await?,
    ))
}

#[utoipa::path(
    get,
    path = "/dictionaries/{dictId}/words/{wordName}",
    params(("dictId" = String, Path), ("wordName" = String, Path)),
    responses(
        (status = 200, body = crate::domains::dictionary::dto::dictionary_dto::WordWithIndexDto),
        (status = 404, body = crate::common::error::ErrorResponse)
    ),
    tag = "Dictionaries"
)]
pub async fn get_dictionary_word(
    State(state): State<AppState>,
    Path((dict_id, word_name)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(
        state
            .dictionary_service
            .get_word(dict_id, word_name)
            .await?,
    ))
}

#[utoipa::path(
    post,
    path = "/dictionaries",
    request_body = DictionaryCreateDto,
    responses(
        (status = 201, body = crate::domains::dictionary::dto::dictionary_dto::DictionaryDto),
        (status = 403, body = crate::common::error::ErrorResponse),
        (status = 409, body = crate::common::error::ErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tag = "Dictionaries"
)]
pub async fn create_dictionary(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    payload: Result<Json<DictionaryCreateDto>, JsonRejection>,
) -> Result<impl IntoResponse, AppError> {
    let Json(payload) = payload.map_err(AppError::from)?;
    let dictionary = state.dictionary_service.create(claims.sub, payload).await?;
    Ok((StatusCode::CREATED, Json(dictionary)))
}

#[utoipa::path(
    put,
    path = "/dictionaries/{dictId}",
    params(("dictId" = String, Path)),
    request_body = DictionaryCreateDto,
    responses(
        (status = 200, body = crate::domains::dictionary::dto::dictionary_dto::DictionaryDto),
        (status = 403, body = crate::common::error::ErrorResponse),
        (status = 404, body = crate::common::error::ErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tag = "Dictionaries"
)]
pub async fn update_dictionary(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(dict_id): Path<String>,
    payload: Result<Json<DictionaryCreateDto>, JsonRejection>,
) -> Result<impl IntoResponse, AppError> {
    let Json(payload) = payload.map_err(AppError::from)?;
    Ok(Json(
        state
            .dictionary_service
            .update(claims.sub, dict_id, payload)
            .await?,
    ))
}

#[utoipa::path(
    post,
    path = "/dictionaries/{dictId}/words:bulk",
    params(("dictId" = String, Path)),
    request_body = WordBulkRequest,
    responses(
        (status = 200, body = crate::domains::dictionary::dto::dictionary_dto::WordBulkResponse),
        (status = 403, body = crate::common::error::ErrorResponse),
        (status = 404, body = crate::common::error::ErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tag = "Dictionaries"
)]
pub async fn bulk_upsert_words(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(dict_id): Path<String>,
    payload: Result<Json<WordBulkRequest>, JsonRejection>,
) -> Result<impl IntoResponse, AppError> {
    let Json(payload) = payload.map_err(AppError::from)?;
    Ok(Json(
        state
            .dictionary_service
            .bulk_words(claims.sub, dict_id, payload)
            .await?,
    ))
}

#[utoipa::path(
    delete,
    path = "/dictionaries/{dictId}",
    params(("dictId" = String, Path)),
    responses(
        (status = 204),
        (status = 403, body = crate::common::error::ErrorResponse),
        (status = 404, body = crate::common::error::ErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tag = "Dictionaries"
)]
pub async fn delete_dictionary(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(dict_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    state.dictionary_service.delete(claims.sub, dict_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
