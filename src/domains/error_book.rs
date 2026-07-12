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
    pub mod error_book_dto;
}

mod infra {
    mod impl_repository;
    pub mod impl_service;
}

pub use api::routes::{error_book_routes, ErrorBookApiDoc};
pub use domain::service::ErrorBookServiceTrait;
pub use infra::impl_service::ErrorBookService;
