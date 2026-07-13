use std::sync::Arc;

use crate::domains::{
    auth::AuthServiceTrait, device::DeviceServiceTrait, dictionary::DictionaryServiceTrait,
    error_book::ErrorBookServiceTrait, file::FileServiceTrait, image::ImageServiceTrait,
    record::RecordServiceTrait, review::ReviewServiceTrait, settings::SettingsServiceTrait,
    stats::StatsServiceTrait, user::UserServiceTrait,
};

use super::config::Config;

/// AppState is a struct that holds the application-wide shared state.
/// It is passed to request handlers via Axum's extension mechanism.
#[derive(Clone)]
pub struct AppState {
    /// Global application configuration.
    pub config: Config,
    /// Service handling authentication-related logic.
    pub auth_service: Arc<dyn AuthServiceTrait>,
    /// Service handling user-related logic.
    pub user_service: Arc<dyn UserServiceTrait>,
    /// Service handling device-related logic.
    pub device_service: Arc<dyn DeviceServiceTrait>,
    /// Service handling dictionaries and words.
    pub dictionary_service: Arc<dyn DictionaryServiceTrait>,
    /// Service handling file-related logic.
    pub file_service: Arc<dyn FileServiceTrait>,
    /// Service handling image generation.
    pub image_service: Arc<dyn ImageServiceTrait>,
    /// Service handling exercise records.
    pub record_service: Arc<dyn RecordServiceTrait>,
    /// Service handling smart review sessions.
    pub review_service: Arc<dyn ReviewServiceTrait>,
    /// Service handling aggregated wrong-word records.
    pub error_book_service: Arc<dyn ErrorBookServiceTrait>,
    /// Service handling practice statistics and analysis.
    pub stats_service: Arc<dyn StatsServiceTrait>,
    /// Service handling per-user application settings.
    pub settings_service: Arc<dyn SettingsServiceTrait>,
}
