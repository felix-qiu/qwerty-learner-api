use axum::{
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    OpenApi,
};

use crate::{
    common::{app_state::AppState, jwt},
    domains::dictionary::{
        api::handlers,
        domain::model::{LanguageCategoryType, LanguageType},
        dto::dictionary_dto::{
            DictionaryCreateDto, DictionaryDto, DictionaryListResponse, WordBulkMode,
            WordBulkRequest, WordBulkResponse, WordDto, WordListResponse, WordWithIndexDto,
        },
    },
};

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::list_dictionaries,
        handlers::get_dictionary,
        handlers::get_dictionary_words,
        handlers::get_dictionary_word,
        handlers::create_dictionary,
        handlers::update_dictionary,
        handlers::bulk_upsert_words,
        handlers::delete_dictionary,
    ),
    components(schemas(
        DictionaryDto,
        DictionaryCreateDto,
        DictionaryListResponse,
        WordDto,
        WordWithIndexDto,
        WordListResponse,
        WordBulkMode,
        WordBulkRequest,
        WordBulkResponse,
        LanguageType,
        LanguageCategoryType,
        crate::common::error::ErrorResponse,
    )),
    tags((name = "Dictionaries", description = "Dictionary and word endpoints")),
    modifiers(&DictionaryApiDoc)
)]
pub struct DictionaryApiDoc;

impl utoipa::Modify for DictionaryApiDoc {
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

pub fn dictionary_routes() -> Router<AppState> {
    let public_routes = Router::new()
        .route("/", get(handlers::list_dictionaries))
        .route("/{dict_id}", get(handlers::get_dictionary))
        .route("/{dict_id}/words", get(handlers::get_dictionary_words))
        .route(
            "/{dict_id}/words/{word_name}",
            get(handlers::get_dictionary_word),
        );

    let admin_routes = Router::new()
        .route("/", post(handlers::create_dictionary))
        .route("/{dict_id}", put(handlers::update_dictionary))
        .route("/{dict_id}", delete(handlers::delete_dictionary))
        .route("/{dict_id}/words:bulk", post(handlers::bulk_upsert_words))
        .route_layer(middleware::from_fn(jwt::jwt_auth));

    public_routes.merge(admin_routes)
}
