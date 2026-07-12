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
    pub mod record_dto;
}

mod infra {
    mod impl_repository;
    pub mod impl_service;
}

pub use api::routes::{record_routes, RecordApiDoc};
pub use domain::service::RecordServiceTrait;
pub use infra::impl_service::RecordService;
