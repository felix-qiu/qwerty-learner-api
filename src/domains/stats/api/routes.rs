use axum::{middleware, routing::get, Router};
use utoipa::{
    openapi::{
        schema::{ArrayItems, Object, Schema, SchemaType},
        security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
        RefOr,
    },
    OpenApi,
};

use crate::{
    common::{app_state::AppState, jwt},
    domains::stats::{
        api::handlers,
        dto::stats_dto::{
            ActivityDayDto, AnalysisStatsDto, ChapterStatsDto, DictStatsDto, StatsSummaryDto,
        },
    },
};

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::get_chapter_stats,
        handlers::get_dictionary_stats,
        handlers::get_analysis_stats,
        handlers::get_stats_summary,
    ),
    components(schemas(
        ChapterStatsDto,
        DictStatsDto,
        ActivityDayDto,
        AnalysisStatsDto,
        StatsSummaryDto,
        crate::common::error::ErrorResponse,
    )),
    tags((name = "Stats", description = "Practice statistics and analysis")),
    modifiers(&StatsApiDoc)
)]
pub struct StatsApiDoc;

impl utoipa::Modify for StatsApiDoc {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        openapi.servers = Some(vec![utoipa::openapi::Server::new("/api/v1")]);
        let components = openapi.components.as_mut().unwrap();
        components.add_security_scheme(
            "bearerAuth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .description(Some("Input your access token"))
                    .build(),
            ),
        );
        set_date_metric_bounds(components, "wpmRecord");
        set_date_metric_bounds(components, "accuracyRecord");
    }
}

fn set_date_metric_bounds(components: &mut utoipa::openapi::Components, property: &str) {
    let Some(RefOr::T(Schema::Object(analysis))) = components.schemas.get_mut("AnalysisStats")
    else {
        return;
    };
    let Some(RefOr::T(Schema::Array(metrics))) = analysis.properties.get_mut(property) else {
        return;
    };
    let ArrayItems::RefOrSchema(metric) = &mut metrics.items else {
        return;
    };
    let RefOr::T(Schema::Array(metric)) = metric.as_mut() else {
        return;
    };
    metric.min_items = Some(2);
    metric.max_items = Some(2);
    metric.prefix_items.clear();
    let mut any_item = Object::new();
    any_item.schema_type = SchemaType::AnyValue;
    metric.items = ArrayItems::RefOrSchema(Box::new(Schema::Object(any_item).into()));
}

pub fn stats_routes() -> Router<AppState> {
    Router::new()
        .route("/stats/chapters", get(handlers::get_chapter_stats))
        .route(
            "/stats/dictionaries/{dict_id}",
            get(handlers::get_dictionary_stats),
        )
        .route("/stats/analysis", get(handlers::get_analysis_stats))
        .route("/stats/summary", get(handlers::get_stats_summary))
        .route_layer(middleware::from_fn(jwt::jwt_auth))
}
