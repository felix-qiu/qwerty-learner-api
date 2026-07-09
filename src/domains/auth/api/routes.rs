use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    OpenApi,
};

use crate::{
    common::{app_state::AppState, jwt},
    domains::auth::{
        api::handlers,
        dto::auth_dto::{
            AuthResponse, AuthUserDto, LoginRequest, RefreshTokenRequest, RegisterRequest,
        },
    },
};

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::register,
        handlers::login,
        handlers::refresh,
        handlers::me,
        handlers::logout,
    ),
    components(schemas(
        AuthUserDto,
        RegisterRequest,
        LoginRequest,
        RefreshTokenRequest,
        AuthResponse,
        crate::common::error::ErrorResponse,
    )),
    tags(
        (name = "Auth", description = "Authentication endpoints")
    ),
    modifiers(&UserAuthApiDoc)
)]
pub struct UserAuthApiDoc;

impl utoipa::Modify for UserAuthApiDoc {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap();
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .description(Some("Input your access token"))
                    .build(),
            ),
        )
    }
}

pub fn user_auth_routes() -> Router<AppState> {
    let public_routes = Router::new()
        .route("/register", post(handlers::register))
        .route("/login", post(handlers::login))
        .route("/refresh", post(handlers::refresh));

    let protected_routes = Router::new()
        .route("/me", get(handlers::me))
        .route("/logout", post(handlers::logout))
        .route_layer(middleware::from_fn(jwt::jwt_auth));

    public_routes.merge(protected_routes)
}
