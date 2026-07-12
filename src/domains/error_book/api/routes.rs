use axum::{middleware, routing::get, Router};
use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    OpenApi,
};

use crate::{
    common::{app_state::AppState, jwt},
    domains::{
        dictionary::dto::dictionary_dto::WordDto,
        error_book::{
            api::handlers,
            dto::error_book_dto::{
                DictErrorWordsResponse, ErrorBookListResponse, ErrorWordDataDto,
            },
        },
        record::dto::record_dto::{LetterMistakes, WordRecordDto},
    },
};

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::list_error_book,
        handlers::get_dictionary_error_words,
    ),
    components(schemas(
        WordDto,
        LetterMistakes,
        WordRecordDto,
        ErrorWordDataDto,
        ErrorBookListResponse,
        DictErrorWordsResponse,
        crate::common::error::ErrorResponse,
    )),
    tags((name = "ErrorBook", description = "Aggregated wrong-word records")),
    modifiers(&ErrorBookApiDoc)
)]
pub struct ErrorBookApiDoc;

impl utoipa::Modify for ErrorBookApiDoc {
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

pub fn error_book_routes() -> Router<AppState> {
    Router::new()
        .route("/error-book", get(handlers::list_error_book))
        .route(
            "/dictionaries/{dict_id}/error-words",
            get(handlers::get_dictionary_error_words),
        )
        .route_layer(middleware::from_fn(jwt::jwt_auth))
}
