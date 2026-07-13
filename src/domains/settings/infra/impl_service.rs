use std::sync::Arc;

use async_trait::async_trait;
use chrono::DateTime;
use serde_json::Value;
use sqlx::PgPool;

use crate::{
    common::error::AppError,
    domains::settings::{
        domain::{repository::SettingsRepository, service::SettingsServiceTrait},
        dto::settings_dto::UserSettingsDto,
        infra::impl_repository::SettingsRepo,
    },
};

#[derive(Clone)]
pub struct SettingsService {
    pool: PgPool,
    repo: Arc<dyn SettingsRepository>,
}

#[async_trait]
impl SettingsServiceTrait for SettingsService {
    fn create_service(pool: PgPool) -> Arc<dyn SettingsServiceTrait> {
        Arc::new(Self {
            pool,
            repo: Arc::new(SettingsRepo),
        })
    }

    async fn get_settings(&self, user_id: String) -> Result<UserSettingsDto, AppError> {
        self.ensure_active_user(&user_id).await?;
        let settings = self
            .repo
            .get_or_create(&self.pool, &user_id)
            .await
            .map_err(db_error)?;
        decode_stored_settings(settings)
    }

    async fn put_settings(
        &self,
        user_id: String,
        settings: UserSettingsDto,
    ) -> Result<UserSettingsDto, AppError> {
        self.ensure_active_user(&user_id).await?;
        validate_settings(&settings)?;
        let value = serde_json::to_value(&settings).map_err(|_| AppError::InternalError)?;
        let settings = self
            .repo
            .replace(&self.pool, &user_id, value)
            .await
            .map_err(db_error)?;
        decode_stored_settings(settings)
    }

    async fn patch_settings(
        &self,
        user_id: String,
        patch: Value,
    ) -> Result<UserSettingsDto, AppError> {
        if !patch.is_object() {
            return Err(AppError::ValidationError(
                "settings patch must be a JSON object".into(),
            ));
        }
        self.ensure_active_user(&user_id).await?;
        let mut transaction = self.pool.begin().await.map_err(db_error)?;
        let mut current = self
            .repo
            .get_for_update(&mut transaction, &user_id)
            .await
            .map_err(db_error)?;
        deep_merge(&mut current, patch);
        let settings = parse_settings(current)?;
        let value = serde_json::to_value(&settings).map_err(|_| AppError::InternalError)?;
        let settings = self
            .repo
            .update(&mut transaction, &user_id, value)
            .await
            .map_err(db_error)?;
        transaction.commit().await.map_err(db_error)?;
        decode_stored_settings(settings)
    }
}

impl SettingsService {
    async fn ensure_active_user(&self, user_id: &str) -> Result<(), AppError> {
        if self
            .repo
            .is_active_user(&self.pool, user_id)
            .await
            .map_err(db_error)?
        {
            Ok(())
        } else {
            Err(AppError::InvalidToken)
        }
    }
}

fn parse_settings(value: Value) -> Result<UserSettingsDto, AppError> {
    let settings = serde_json::from_value::<UserSettingsDto>(value)
        .map_err(|error| AppError::ValidationError(error.to_string()))?;
    validate_settings(&settings)?;
    Ok(settings)
}

fn decode_stored_settings(value: Value) -> Result<UserSettingsDto, AppError> {
    let settings = serde_json::from_value::<UserSettingsDto>(value).map_err(|error| {
        tracing::error!(?error, "stored user settings are invalid");
        AppError::InternalError
    })?;
    validate_settings(&settings).map_err(|error| {
        tracing::error!(?error, "stored user settings failed validation");
        AppError::InternalError
    })?;
    Ok(settings)
}

fn validate_settings(settings: &UserSettingsDto) -> Result<(), AppError> {
    if settings
        .dismiss_start_card_date
        .0
        .as_deref()
        .is_some_and(|value| DateTime::parse_from_rfc3339(value).is_err())
    {
        return Err(AppError::ValidationError(
            "dismissStartCardDate must be an RFC3339 date-time or null".into(),
        ));
    }
    Ok(())
}

fn deep_merge(target: &mut Value, patch: Value) {
    match (target, patch) {
        (Value::Object(target), Value::Object(patch)) => {
            for (key, value) in patch {
                if let Some(target) = target.get_mut(&key) {
                    deep_merge(target, value);
                } else {
                    target.insert(key, value);
                }
            }
        }
        (target, patch) => *target = patch,
    }
}

fn db_error(error: sqlx::Error) -> AppError {
    tracing::error!(?error, "settings database operation failed");
    AppError::InternalError
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::{deep_merge, parse_settings};

    #[test]
    fn deep_merge_preserves_unpatched_fields() {
        let mut current = default_settings();
        deep_merge(
            &mut current,
            json!({"pronunciation": {"type": "uk", "name": "英音"}}),
        );
        assert_eq!(current["pronunciation"]["type"], "uk");
        assert_eq!(current["pronunciation"]["name"], "英音");
        assert_eq!(current["pronunciation"]["volume"], 1);
        assert!(parse_settings(current).is_ok());
    }

    #[test]
    fn validation_preserves_additional_properties() {
        let mut unknown = default_settings();
        unknown["unknown"] = json!(true);
        unknown["pronunciation"]["futureOption"] = json!({"enabled": true});
        let parsed = parse_settings(unknown).unwrap();
        assert_eq!(parsed.extra.get("unknown"), Some(&json!(true)));
        assert_eq!(
            parsed.pronunciation.extra.get("futureOption"),
            Some(&json!({"enabled": true}))
        );
        let serialized = serde_json::to_value(parsed).unwrap();
        assert_eq!(serialized["unknown"], true);
        assert_eq!(
            serialized["pronunciation"]["futureOption"],
            json!({"enabled": true})
        );
    }

    #[test]
    fn validation_rejects_invalid_null_enum_and_date() {
        let mut invalid_null = default_settings();
        invalid_null["currentDict"] = Value::Null;
        assert!(parse_settings(invalid_null).is_err());

        let mut invalid_enum = default_settings();
        invalid_enum["pronunciation"]["type"] = json!("invalid");
        assert!(parse_settings(invalid_enum).is_err());

        let mut invalid_date = default_settings();
        invalid_date["dismissStartCardDate"] = json!("2026-07-13");
        assert!(parse_settings(invalid_date).is_err());
    }

    #[test]
    fn nullable_fields_accept_null() {
        let settings = default_settings();
        assert!(parse_settings(settings).is_ok());
    }

    fn default_settings() -> Value {
        json!({
            "currentDict": "cet4",
            "currentChapter": 0,
            "loopWordConfig": {"times": 1},
            "keySoundsConfig": {
                "isOpen": true,
                "isOpenClickSound": true,
                "volume": 1,
                "resource": null
            },
            "hintSoundsConfig": {
                "isOpen": true,
                "volume": 1,
                "isOpenWrongSound": true,
                "isOpenCorrectSound": true,
                "wrongResource": null,
                "correctResource": null
            },
            "pronunciation": {
                "isOpen": true,
                "volume": 1,
                "type": "us",
                "name": "美音",
                "isLoop": false,
                "isTransRead": false,
                "transVolume": 1,
                "rate": 1
            },
            "fontsize": {"foreignFont": 48, "translateFont": 18},
            "randomConfig": {"isOpen": false},
            "phoneticConfig": {"isOpen": true, "type": "us"},
            "wordDictationConfig": {"isOpen": false, "type": "hideAll", "openBy": "auto"},
            "isShowPrevAndNextWord": true,
            "isIgnoreCase": true,
            "isShowAnswerOnHover": true,
            "isTextSelectable": false,
            "isOpenDarkMode": false,
            "dismissStartCardDate": null,
            "hasSeenEnhancedPromotion": false
        })
    }
}
