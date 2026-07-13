mod api {
    mod handlers;
    pub mod routes;
}

mod domain {
    pub mod repository;
    pub mod service;
}

pub mod dto {
    pub mod settings_dto;
}

mod infra {
    mod impl_repository;
    pub mod impl_service;
}

pub use api::routes::{settings_routes, SettingsApiDoc};
pub use domain::service::SettingsServiceTrait;
pub use infra::impl_service::SettingsService;
