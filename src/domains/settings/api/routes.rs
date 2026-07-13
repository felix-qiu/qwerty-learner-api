use axum::{middleware, routing::get, Router};
use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    OpenApi,
};

use crate::{
    common::{app_state::AppState, jwt},
    domains::settings::{
        api::handlers,
        dto::settings_dto::{
            PhoneticType, PronunciationType, SoundResourceDto, UserSettingsDto,
            UserSettingsPatchDto, WordDictationOpenBy, WordDictationType,
        },
    },
};

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::get_settings,
        handlers::put_settings,
        handlers::patch_settings,
    ),
    components(schemas(
        PronunciationType,
        PhoneticType,
        WordDictationType,
        WordDictationOpenBy,
        SoundResourceDto,
        UserSettingsDto,
        UserSettingsPatchDto,
        crate::common::error::ErrorResponse,
    )),
    tags((name = "Settings", description = "Per-user application settings")),
    modifiers(&SettingsApiDoc)
)]
pub struct SettingsApiDoc;

impl utoipa::Modify for SettingsApiDoc {
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

pub fn settings_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/settings",
            get(handlers::get_settings)
                .put(handlers::put_settings)
                .patch(handlers::patch_settings),
        )
        .route_layer(middleware::from_fn(jwt::jwt_auth))
}
