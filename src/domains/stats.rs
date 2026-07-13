mod api {
    mod handlers;
    pub mod routes;
}

mod domain {
    pub mod model;
    pub mod repository;
    pub mod service;
}

pub mod dto {
    pub mod stats_dto;
}

mod infra {
    mod impl_repository;
    pub mod impl_service;
}

pub use api::routes::{stats_routes, StatsApiDoc};
pub use domain::service::StatsServiceTrait;
pub use infra::impl_service::StatsService;
