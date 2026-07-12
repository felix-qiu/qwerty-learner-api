use axum::{
    extract::{
        rejection::{JsonRejection, PathRejection, QueryRejection},
        Extension, Path, Query, State,
    },
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::{
    common::{app_state::AppState, error::AppError, jwt::Claims},
    domains::review::dto::review_dto::{
        LatestReviewQuery, ReviewCreateDto, ReviewListQuery, ReviewUpdateDto,
    },
};

#[utoipa::path(
    get,
    path = "/reviews",
    operation_id = "listReviews",
    summary = "复习记录列表",
    params(ReviewListQuery),
    responses((status = 200, description = "OK", body = crate::domains::review::dto::review_dto::ReviewListResponse)),
    security(("bearerAuth" = [])),
    tag = "Reviews"
)]
pub async fn list_reviews(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    query: Result<Query<ReviewListQuery>, QueryRejection>,
) -> Result<impl IntoResponse, AppError> {
    let Query(query) = query.map_err(AppError::from)?;
    Ok(Json(
        state.review_service.list_reviews(claims.sub, query).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/reviews",
    operation_id = "createReview",
    summary = "基于错题生成新复习",
    description = "排序：errorCountScore*0.6 + latestErrorTimeScore*0.4，升序。\n与 `src/utils/db/review-record.ts` 一致。",
    request_body = ReviewCreateDto,
    responses((status = 201, description = "Created", body = crate::domains::review::dto::review_dto::ReviewRecordDto)),
    security(("bearerAuth" = [])),
    tag = "Reviews"
)]
pub async fn create_review(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    payload: Result<Json<ReviewCreateDto>, JsonRejection>,
) -> Result<impl IntoResponse, AppError> {
    let Json(payload) = payload.map_err(AppError::from)?;
    let record = state
        .review_service
        .create_review(claims.sub, payload)
        .await?;
    Ok((StatusCode::CREATED, Json(record)))
}

#[utoipa::path(
    get,
    path = "/reviews/latest",
    operation_id = "getLatestReview",
    summary = "最新未完成复习",
    params(LatestReviewQuery),
    responses((status = 200, description = "OK（无未完成复习时 record 为 null）", body = crate::domains::review::dto::review_dto::LatestReviewResponse)),
    security(("bearerAuth" = [])),
    tag = "Reviews"
)]
pub async fn latest_review(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    query: Result<Query<LatestReviewQuery>, QueryRejection>,
) -> Result<impl IntoResponse, AppError> {
    let Query(query) = query.map_err(AppError::from)?;
    Ok(Json(
        state
            .review_service
            .latest_review(claims.sub, query)
            .await?,
    ))
}

#[utoipa::path(
    patch,
    path = "/reviews/{reviewId}",
    operation_id = "updateReview",
    summary = "更新复习进度",
    params(("reviewId" = i64, Path, minimum = 1)),
    request_body = ReviewUpdateDto,
    responses(
        (status = 200, description = "OK", body = crate::domains::review::dto::review_dto::ReviewRecordDto),
        (status = 404, description = "Not Found", body = crate::common::error::ErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tag = "Reviews"
)]
pub async fn update_review(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    review_id: Result<Path<i64>, PathRejection>,
    payload: Result<Json<ReviewUpdateDto>, JsonRejection>,
) -> Result<impl IntoResponse, AppError> {
    let Json(payload) = payload.map_err(AppError::from)?;
    let Path(review_id) =
        review_id.map_err(|rejection| AppError::BadRequest(rejection.body_text()))?;
    Ok(Json(
        state
            .review_service
            .update_review(claims.sub, review_id, payload)
            .await?,
    ))
}
