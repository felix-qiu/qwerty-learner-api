use axum::{
    extract::{rejection::QueryRejection, Extension, Path, Query, State},
    response::IntoResponse,
    Json,
};

use crate::{
    common::{app_state::AppState, error::AppError, jwt::Claims},
    domains::stats::dto::stats_dto::{AnalysisStatsQuery, ChapterStatsQuery},
};

#[utoipa::path(
    get,
    path = "/stats/chapters",
    operation_id = "getChapterStats",
    summary = "章节统计",
    params(ChapterStatsQuery),
    responses((status = 200, description = "OK", body = crate::domains::stats::dto::stats_dto::ChapterStatsDto)),
    security(("bearerAuth" = [])),
    tag = "Stats"
)]
pub async fn get_chapter_stats(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    query: Result<Query<ChapterStatsQuery>, QueryRejection>,
) -> Result<impl IntoResponse, AppError> {
    let Query(query) = query.map_err(AppError::from)?;
    Ok(Json(
        state
            .stats_service
            .get_chapter_stats(claims.sub, query)
            .await?,
    ))
}

#[utoipa::path(
    get,
    path = "/stats/dictionaries/{dictId}",
    operation_id = "getDictStats",
    summary = "词典练习进度",
    params(("dictId" = String, Path, example = "cet4")),
    responses((status = 200, description = "OK", body = crate::domains::stats::dto::stats_dto::DictStatsDto)),
    security(("bearerAuth" = [])),
    tag = "Stats"
)]
pub async fn get_dictionary_stats(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(dict_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(
        state
            .stats_service
            .get_dictionary_stats(claims.sub, dict_id)
            .await?,
    ))
}

#[utoipa::path(
    get,
    path = "/stats/analysis",
    operation_id = "getAnalysisStats",
    summary = "分析页总览",
    params(AnalysisStatsQuery),
    responses((status = 200, description = "OK", body = crate::domains::stats::dto::stats_dto::AnalysisStatsDto)),
    security(("bearerAuth" = [])),
    tag = "Stats"
)]
pub async fn get_analysis_stats(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    query: Result<Query<AnalysisStatsQuery>, QueryRejection>,
) -> Result<impl IntoResponse, AppError> {
    let Query(query) = query.map_err(AppError::from)?;
    Ok(Json(
        state
            .stats_service
            .get_analysis_stats(claims.sub, query)
            .await?,
    ))
}

#[utoipa::path(
    get,
    path = "/stats/summary",
    operation_id = "getStatsSummary",
    summary = "用户总览",
    responses((status = 200, description = "OK", body = crate::domains::stats::dto::stats_dto::StatsSummaryDto)),
    security(("bearerAuth" = [])),
    tag = "Stats"
)]
pub async fn get_stats_summary(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(
        state.stats_service.get_stats_summary(claims.sub).await?,
    ))
}
