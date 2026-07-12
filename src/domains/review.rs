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
    pub mod review_dto;
}

mod infra {
    mod impl_repository;
    pub mod impl_service;
}

pub use api::routes::{review_routes, ReviewApiDoc};
pub use domain::service::ReviewServiceTrait;
pub use infra::impl_service::ReviewService;
