use axum::{
    extract::{
        rejection::{JsonRejection, QueryRejection},
        Extension, Query, State,
    },
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::{
    common::{app_state::AppState, error::AppError, jwt::Claims},
    domains::record::dto::record_dto::{
        ChapterRecordCreateDto, ChapterRecordListQuery, DeleteWordRecordsQuery,
        WordRecordBatchRequest, WordRecordCreateDto, WordRecordListQuery,
    },
};

#[utoipa::path(
    post,
    path = "/records/words",
    operation_id = "createWordRecord",
    summary = "提交单词练习记录",
    request_body = WordRecordCreateDto,
    responses((status = 201, body = crate::domains::record::dto::record_dto::WordRecordDto)),
    security(("bearerAuth" = [])),
    tag = "Records"
)]
pub async fn create_word_record(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    payload: Result<Json<WordRecordCreateDto>, JsonRejection>,
) -> Result<impl IntoResponse, AppError> {
    let Json(payload) = payload.map_err(AppError::from)?;
    let record = state
        .record_service
        .create_word_record(claims.sub, payload)
        .await?;
    Ok((StatusCode::CREATED, Json(record)))
}

#[utoipa::path(
    post,
    path = "/records/words:batch",
    operation_id = "batchCreateWordRecords",
    summary = "批量提交单词记录（离线同步）",
    request_body = WordRecordBatchRequest,
    responses((status = 201, body = crate::domains::record::dto::record_dto::WordRecordBatchResponse)),
    security(("bearerAuth" = [])),
    tag = "Records"
)]
pub async fn batch_create_word_records(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    payload: Result<Json<WordRecordBatchRequest>, JsonRejection>,
) -> Result<impl IntoResponse, AppError> {
    let Json(payload) = payload.map_err(AppError::from)?;
    let response = state
        .record_service
        .batch_create_word_records(claims.sub, payload)
        .await?;
    Ok((StatusCode::CREATED, Json(response)))
}

#[utoipa::path(
    get,
    path = "/records/words",
    operation_id = "listWordRecords",
    summary = "查询单词记录",
    params(WordRecordListQuery),
    responses((status = 200, body = crate::domains::record::dto::record_dto::WordRecordListResponse)),
    security(("bearerAuth" = [])),
    tag = "Records"
)]
pub async fn list_word_records(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    query: Result<Query<WordRecordListQuery>, QueryRejection>,
) -> Result<impl IntoResponse, AppError> {
    let Query(query) = query.map_err(AppError::from)?;
    Ok(Json(
        state
            .record_service
            .list_word_records(claims.sub, query)
            .await?,
    ))
}

#[utoipa::path(
    delete,
    path = "/records/words",
    operation_id = "deleteWordRecords",
    summary = "删除某词在某词典下的全部记录",
    params(DeleteWordRecordsQuery),
    responses((status = 200, body = crate::domains::record::dto::record_dto::DeleteWordRecordsResponse)),
    security(("bearerAuth" = [])),
    tag = "Records"
)]
pub async fn delete_word_records(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    query: Result<Query<DeleteWordRecordsQuery>, QueryRejection>,
) -> Result<impl IntoResponse, AppError> {
    let Query(query) = query.map_err(AppError::from)?;
    Ok(Json(
        state
            .record_service
            .delete_word_records(claims.sub, query)
            .await?,
    ))
}

#[utoipa::path(
    post,
    path = "/records/chapters",
    operation_id = "createChapterRecord",
    summary = "提交章节练习记录",
    request_body = ChapterRecordCreateDto,
    responses((status = 201, body = crate::domains::record::dto::record_dto::ChapterRecordDto)),
    security(("bearerAuth" = [])),
    tag = "Records"
)]
pub async fn create_chapter_record(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    payload: Result<Json<ChapterRecordCreateDto>, JsonRejection>,
) -> Result<impl IntoResponse, AppError> {
    let Json(payload) = payload.map_err(AppError::from)?;
    let record = state
        .record_service
        .create_chapter_record(claims.sub, payload)
        .await?;
    Ok((StatusCode::CREATED, Json(record)))
}

#[utoipa::path(
    get,
    path = "/records/chapters",
    operation_id = "listChapterRecords",
    summary = "查询章节记录",
    params(ChapterRecordListQuery),
    responses((status = 200, body = crate::domains::record::dto::record_dto::ChapterRecordListResponse)),
    security(("bearerAuth" = [])),
    tag = "Records"
)]
pub async fn list_chapter_records(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    query: Result<Query<ChapterRecordListQuery>, QueryRejection>,
) -> Result<impl IntoResponse, AppError> {
    let Query(query) = query.map_err(AppError::from)?;
    Ok(Json(
        state
            .record_service
            .list_chapter_records(claims.sub, query)
            .await?,
    ))
}
