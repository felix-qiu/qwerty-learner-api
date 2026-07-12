use axum::{middleware, routing::post, Router};
use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    OpenApi,
};

use crate::{
    common::{app_state::AppState, jwt},
    domains::record::{
        api::handlers,
        dto::record_dto::{
            ChapterRecordCreateDto, ChapterRecordDto, ChapterRecordListResponse,
            DeleteWordRecordsResponse, LetterMistakes, WordRecordBatchRequest,
            WordRecordBatchResponse, WordRecordCreateDto, WordRecordDto, WordRecordListResponse,
        },
    },
};

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::create_word_record,
        handlers::batch_create_word_records,
        handlers::list_word_records,
        handlers::delete_word_records,
        handlers::create_chapter_record,
        handlers::list_chapter_records,
    ),
    components(schemas(
        WordRecordCreateDto,
        LetterMistakes,
        WordRecordDto,
        WordRecordBatchRequest,
        WordRecordBatchResponse,
        WordRecordListResponse,
        DeleteWordRecordsResponse,
        ChapterRecordCreateDto,
        ChapterRecordDto,
        ChapterRecordListResponse,
        crate::common::error::ErrorResponse,
    )),
    tags((name = "Records", description = "Word and chapter exercise records")),
    modifiers(&RecordApiDoc)
)]
pub struct RecordApiDoc;

impl utoipa::Modify for RecordApiDoc {
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

pub fn record_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/words",
            post(handlers::create_word_record)
                .get(handlers::list_word_records)
                .delete(handlers::delete_word_records),
        )
        .route("/words:batch", post(handlers::batch_create_word_records))
        .route(
            "/chapters",
            post(handlers::create_chapter_record).get(handlers::list_chapter_records),
        )
        .route_layer(middleware::from_fn(jwt::jwt_auth))
}
