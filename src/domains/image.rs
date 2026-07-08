mod api {
    mod handlers;
    pub mod routes;
}

mod domain {
    pub mod service;
}

pub mod dto {
    pub mod image_dto;
}

mod infra {
    pub mod impl_service;
}

pub use api::routes::{image_routes, ImageApiDoc};
pub use domain::service::ImageServiceTrait;
pub use infra::impl_service::ImageService;
