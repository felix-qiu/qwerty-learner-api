use super::handlers::*;
use crate::{
    common::app_state::AppState, domains::image::dto::image_dto::GenerateWordImageRequest,
};
use axum::{routing::post, Router};

use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    OpenApi,
};

#[derive(OpenApi)]
#[openapi(
    paths(generate_word_image),
    components(schemas(GenerateWordImageRequest)),
    tags(
        (name = "Images", description = "Image generation endpoints")
    ),
    security(
        ("bearerAuth" = [])
    ),
    modifiers(&ImageApiDoc)
)]
pub struct ImageApiDoc;

impl utoipa::Modify for ImageApiDoc {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap();
        components.add_security_scheme(
            "bearerAuth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .description(Some("Input your `<your‑jwt>`"))
                    .build(),
            ),
        )
    }
}

pub fn image_routes() -> Router<AppState> {
    Router::new().route("/word", post(generate_word_image))
}
