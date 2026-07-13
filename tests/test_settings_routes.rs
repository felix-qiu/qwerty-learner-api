use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use clean_axum_demo::{
    app::create_router,
    common::{
        bootstrap::build_app_state, config::Config, error::ErrorResponse, jwt::make_access_token,
    },
    domains::settings::{dto::settings_dto::UserSettingsDto, SettingsApiDoc},
};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
use utoipa::OpenApi;

mod test_helpers;

use test_helpers::{
    deserialize_json_body, request_with_auth, request_with_auth_and_body, TEST_USER_ID,
};

#[tokio::test]
async fn test_settings_contract_authentication_and_rejections() {
    dotenvy::from_filename(".env.test").expect("Failed to load .env.test");
    let config = Config::from_env().unwrap();
    let pool = PgPoolOptions::new()
        .connect_lazy(&config.database_url)
        .unwrap();
    let router = create_router(build_app_state(pool, config));

    let openapi = SettingsApiDoc::openapi();
    assert_eq!(openapi.servers.as_ref().unwrap()[0].url, "/api/v1");
    assert!(openapi
        .components
        .as_ref()
        .unwrap()
        .security_schemes
        .contains_key("bearerAuth"));
    let openapi_json = serde_json::to_value(&openapi).unwrap();
    for (method, operation_id) in [
        ("get", "getSettings"),
        ("put", "putSettings"),
        ("patch", "patchSettings"),
    ] {
        let path = format!("/paths/~1settings/{method}");
        assert_eq!(
            openapi_json
                .pointer(&format!("{path}/operationId"))
                .unwrap(),
            operation_id
        );
        assert_eq!(
            openapi_json
                .pointer(&format!("{path}/security/0/bearerAuth"))
                .unwrap(),
            &serde_json::json!([])
        );
        assert_eq!(
            openapi_json
                .pointer(&format!(
                    "{path}/responses/200/content/application~1json/schema/$ref"
                ))
                .unwrap(),
            "#/components/schemas/UserSettings"
        );
    }
    assert!(openapi_json
        .pointer("/components/schemas/UserSettings/required")
        .is_none());
    assert_eq!(
        openapi_json
            .pointer("/paths/~1settings/put/requestBody/content/application~1json/schema/$ref")
            .unwrap(),
        "#/components/schemas/UserSettings"
    );
    assert_eq!(
        openapi_json
            .pointer("/paths/~1settings/patch/requestBody/content/application~1json/schema/$ref")
            .unwrap(),
        "#/components/schemas/UserSettingsPatch"
    );
    assert_eq!(
        openapi_json
            .pointer("/components/schemas/UserSettingsPatch/additionalProperties")
            .unwrap(),
        &serde_json::json!({})
    );
    assert!(openapi_json
        .pointer("/components/schemas/UserSettings/properties/keySoundsConfig/properties/resource")
        .unwrap()
        .to_string()
        .contains("null"));
    assert_eq!(
        openapi_json
            .pointer("/components/schemas/UserSettings/properties/hintSoundsConfig/properties/isOpenWrongSound/type")
            .unwrap(),
        "boolean"
    );
    assert_eq!(
        openapi_json
            .pointer("/components/schemas/UserSettings/properties/dismissStartCardDate/format")
            .unwrap(),
        "date-time"
    );
    assert_eq!(
        openapi_json
            .pointer("/components/schemas/PronunciationType/enum")
            .unwrap(),
        &serde_json::json!(["us", "uk", "romaji", "zh", "ja", "de", "hapin", "kk", "id"])
    );
    assert_eq!(
        openapi_json
            .pointer("/components/schemas/WordDictationType/enum")
            .unwrap(),
        &serde_json::json!(["hideAll", "hideVowel", "hideConsonant", "randomHide"])
    );

    for method in [Method::GET, Method::PUT, Method::PATCH] {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri("/api/v1/settings")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    let token = make_access_token(TEST_USER_ID).unwrap();
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri("/api/v1/settings")
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"currentDict":"cet4"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::UNPROCESSABLE_ENTITY);
    let error: ErrorResponse = deserialize_json_body(body).await.unwrap();
    assert_eq!(error.code, "VALIDATION_ERROR");

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri("/api/v1/settings")
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from("{"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri("/api/v1/settings")
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from("[]"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_settings_get_put_and_deep_patch() {
    let original = get_settings().await;

    let mut replacement = serde_json::to_value(&original).unwrap();
    replacement["currentDict"] = serde_json::json!("apitest-dict");
    replacement["currentChapter"] = serde_json::json!(1);
    replacement["keySoundsConfig"]["resource"] = serde_json::json!({
        "key": "Default",
        "name": "Default",
        "filename": "Default.wav"
    });
    replacement["dismissStartCardDate"] = serde_json::json!("2026-07-13T00:00:00+08:00");

    let response = request_with_auth_and_body(Method::PUT, "/api/v1/settings", &replacement).await;
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::OK);
    let replaced: UserSettingsDto = deserialize_json_body(body).await.unwrap();
    assert_eq!(replaced.current_dict, "apitest-dict");
    assert_eq!(replaced.current_chapter, 1);
    assert!(replaced.key_sounds_config.resource.0.is_some());

    let patch = serde_json::json!({
        "currentChapter": 3,
        "pronunciation": {"type": "uk", "name": "英音"},
        "keySoundsConfig": {"resource": null},
        "dismissStartCardDate": null
    });
    let response = request_with_auth_and_body(Method::PATCH, "/api/v1/settings", &patch).await;
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::OK);
    let patched: UserSettingsDto = deserialize_json_body(body).await.unwrap();
    assert_eq!(patched.current_dict, "apitest-dict");
    assert_eq!(patched.current_chapter, 3);
    assert_eq!(
        serde_json::to_value(patched.pronunciation.r#type).unwrap(),
        "uk"
    );
    assert_eq!(patched.pronunciation.name, "英音");
    assert_eq!(patched.pronunciation.volume, original.pronunciation.volume);
    assert!(patched.key_sounds_config.resource.0.is_none());
    assert!(patched.dismiss_start_card_date.0.is_none());

    let response = request_with_auth_and_body(
        Method::PATCH,
        "/api/v1/settings",
        &serde_json::json!({
            "unknown": true,
            "pronunciation": {"futureOption": {"enabled": true}}
        }),
    )
    .await;
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::OK);
    let extended: UserSettingsDto = deserialize_json_body(body).await.unwrap();
    assert_eq!(
        extended.extra.get("unknown"),
        Some(&serde_json::json!(true))
    );
    assert_eq!(
        extended.pronunciation.extra.get("futureOption"),
        Some(&serde_json::json!({"enabled": true}))
    );

    let _ = request_with_auth_and_body(Method::PUT, "/api/v1/settings", &original).await;
}

async fn get_settings() -> UserSettingsDto {
    let response = request_with_auth(Method::GET, "/api/v1/settings").await;
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::OK);
    deserialize_json_body(body).await.unwrap()
}
