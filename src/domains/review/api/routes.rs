use axum::{middleware, routing::get, Router};
use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    OpenApi,
};

use crate::{
    common::{app_state::AppState, jwt},
    domains::{
        dictionary::dto::dictionary_dto::WordDto,
        review::{
            api::handlers,
            dto::review_dto::{
                LatestReviewResponse, ReviewCreateDto, ReviewErrorDataDto, ReviewListResponse,
                ReviewRecordDto, ReviewUpdateDto,
            },
        },
    },
};

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::list_reviews,
        handlers::create_review,
        handlers::latest_review,
        handlers::update_review,
    ),
    components(schemas(
        WordDto,
        ReviewRecordDto,
        ReviewErrorDataDto,
        ReviewCreateDto,
        ReviewUpdateDto,
        ReviewListResponse,
        LatestReviewResponse,
        crate::common::error::ErrorResponse,
    )),
    tags((name = "Reviews", description = "Smart word review sessions")),
    modifiers(&ReviewApiDoc)
)]
pub struct ReviewApiDoc;

impl utoipa::Modify for ReviewApiDoc {
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
    }
}

pub fn review_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(handlers::list_reviews).post(handlers::create_review),
        )
        .route("/latest", get(handlers::latest_review))
        .route(
            "/{review_id}",
            axum::routing::patch(handlers::update_review),
        )
        .route_layer(middleware::from_fn(jwt::jwt_auth))
}
