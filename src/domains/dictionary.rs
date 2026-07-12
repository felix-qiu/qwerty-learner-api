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
    pub mod dictionary_dto;
}

mod infra {
    mod impl_repository;
    pub mod impl_service;
}

pub use api::routes::{dictionary_routes, DictionaryApiDoc};
pub use domain::model::{LanguageCategoryType, LanguageType};
pub use domain::service::DictionaryServiceTrait;
pub use infra::impl_service::DictionaryService;
