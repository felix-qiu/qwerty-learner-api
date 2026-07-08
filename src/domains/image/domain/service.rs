use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    common::{config::Config, error::AppError},
    domains::image::dto::image_dto::GeneratedImageDto,
};

#[async_trait]
pub trait ImageServiceTrait: Send + Sync {
    fn create_service(config: Config) -> Arc<dyn ImageServiceTrait>
    where
        Self: Sized;

    async fn generate_word_image(&self, word: String) -> Result<GeneratedImageDto, AppError>;
}
