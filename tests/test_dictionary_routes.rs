use axum::{
    body::Body,
    http::{header::CONTENT_TYPE, Method, Request, StatusCode},
};

use clean_axum_demo::{
    app::create_router,
    common::error::ErrorResponse,
    common::{bootstrap::build_app_state, config::Config},
    domains::auth::UserAuthApiDoc,
    domains::dictionary::dto::dictionary_dto::{
        DictionaryCreateDto, DictionaryDto, DictionaryListResponse, WordBulkMode, WordBulkRequest,
        WordBulkResponse, WordDto, WordListResponse, WordWithIndexDto,
    },
    domains::dictionary::{DictionaryApiDoc, LanguageCategoryType, LanguageType},
};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
use utoipa::OpenApi;

mod test_helpers;

use test_helpers::{
    deserialize_json_body, request, request_with_admin_auth, request_with_admin_auth_and_body,
    request_with_auth_and_body,
};

#[tokio::test]
async fn test_dictionary_routes_build() {
    dotenvy::from_filename(".env.test").expect("Failed to load .env.test");
    let config = Config::from_env().unwrap();
    let pool = PgPoolOptions::new()
        .connect_lazy(&config.database_url)
        .unwrap();
    let state = build_app_state(pool, config);

    for openapi in [UserAuthApiDoc::openapi(), DictionaryApiDoc::openapi()] {
        assert_eq!(openapi.servers.as_ref().unwrap()[0].url, "/api/v1");
        let components = openapi.components.as_ref().unwrap();
        assert!(components.security_schemes.contains_key("bearerAuth"));
        assert!(!components.security_schemes.contains_key("bearer_auth"));
    }
    let dictionary_openapi = DictionaryApiDoc::openapi();
    let schemas = &dictionary_openapi.components.as_ref().unwrap().schemas;
    assert!(schemas.contains_key("LanguageType"));
    assert!(schemas.contains_key("LanguageCategoryType"));

    let router = create_router(state);

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/auth/login")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::BAD_REQUEST);
    let error: ErrorResponse = deserialize_json_body(body).await.unwrap();
    assert_eq!(error.code, "BAD_REQUEST");

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/dictionaries?languageCategory=invalid")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::BAD_REQUEST);
    let error: ErrorResponse = deserialize_json_body(body).await.unwrap();
    assert_eq!(error.code, "BAD_REQUEST");

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/dictionaries")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_and_get_dictionary() {
    let response = request(
        Method::GET,
        "/api/v1/dictionaries?languageCategory=en&category=%E6%B5%8B%E8%AF%95&tag=test&q=API",
    )
    .await;
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::OK);

    let list: DictionaryListResponse = deserialize_json_body(body).await.unwrap();
    assert_eq!(list.total, 1);
    assert_eq!(list.items[0].id, "apitest-dict");
    assert_eq!(list.items[0].length, 25);
    assert_eq!(list.items[0].chapter_count, 2);

    let response = request(Method::GET, "/api/v1/dictionaries/apitest-dict").await;
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::OK);
    let dictionary: DictionaryDto = deserialize_json_body(body).await.unwrap();
    assert_eq!(dictionary.id, "apitest-dict");
}

#[tokio::test]
async fn test_get_dictionary_words_defaults_to_twenty() {
    let response = request(Method::GET, "/api/v1/dictionaries/apitest-dict/words").await;
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::OK);

    let result: WordListResponse = deserialize_json_body(body).await.unwrap();
    assert_eq!(result.dict_id, "apitest-dict");
    assert_eq!(result.chapter, None);
    assert_eq!(result.total, 25);
    assert_eq!(result.words.len(), 20);
    assert_eq!(result.words[0].index, 0);
    assert_eq!(result.words[19].index, 19);
}

#[tokio::test]
async fn test_get_dictionary_words_by_chapter_and_name() {
    let response = request(
        Method::GET,
        "/api/v1/dictionaries/apitest-dict/words?chapter=1&offset=0&limit=1",
    )
    .await;
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::OK);

    let result: WordListResponse = deserialize_json_body(body).await.unwrap();
    assert_eq!(result.chapter, Some(1));
    assert_eq!(result.words.len(), 5);
    assert_eq!(result.words[0].index, 20);

    let response = request(
        Method::GET,
        "/api/v1/dictionaries/apitest-dict/words/word24",
    )
    .await;
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::OK);
    let word: WordWithIndexDto = deserialize_json_body(body).await.unwrap();
    assert_eq!(word.index, 24);
}

#[tokio::test]
async fn test_dictionary_admin_authorization() {
    let payload = dictionary_payload(format!("forbidden-{}", uuid::Uuid::new_v4()));
    let response = request_with_auth_and_body(Method::POST, "/api/v1/dictionaries", &payload).await;
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::FORBIDDEN);

    let error: ErrorResponse = deserialize_json_body(body).await.unwrap();
    assert_eq!(error.code, "FORBIDDEN");
}

#[tokio::test]
async fn test_dictionary_admin_lifecycle() {
    let dict_id = format!("dict-{}", uuid::Uuid::new_v4());
    let mut payload = dictionary_payload(dict_id.clone());

    let response =
        request_with_admin_auth_and_body(Method::POST, "/api/v1/dictionaries", &payload).await;
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::CREATED);
    let created: DictionaryDto = deserialize_json_body(body).await.unwrap();
    assert_eq!(created.id, dict_id);
    assert_eq!(created.length, 0);

    payload.name = "Updated Dictionary".to_string();
    let uri = format!("/api/v1/dictionaries/{dict_id}");
    let response = request_with_admin_auth_and_body(Method::PUT, &uri, &payload).await;
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::OK);
    let updated: DictionaryDto = deserialize_json_body(body).await.unwrap();
    assert_eq!(updated.name, payload.name);

    let replace = WordBulkRequest {
        mode: WordBulkMode::Replace,
        words: (0..21).map(test_word).collect(),
    };
    let bulk_uri = format!("/api/v1/dictionaries/{dict_id}/words:bulk");
    let response = request_with_admin_auth_and_body(Method::POST, &bulk_uri, &replace).await;
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::OK);
    let result: WordBulkResponse = deserialize_json_body(body).await.unwrap();
    assert_eq!(result.written, 21);
    assert_eq!(result.length, 21);
    assert_eq!(result.chapter_count, 2);

    let append = WordBulkRequest {
        mode: WordBulkMode::Append,
        words: vec![test_word(21)],
    };
    let response = request_with_admin_auth_and_body(Method::POST, &bulk_uri, &append).await;
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::OK);
    let result: WordBulkResponse = deserialize_json_body(body).await.unwrap();
    assert_eq!(result.written, 1);
    assert_eq!(result.length, 22);

    let response = request_with_admin_auth(Method::DELETE, &uri).await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let response = request(Method::GET, &uri).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

fn dictionary_payload(id: String) -> DictionaryCreateDto {
    DictionaryCreateDto {
        id,
        name: "Test Dictionary".to_string(),
        description: "Integration test dictionary".to_string(),
        category: "测试".to_string(),
        tags: vec!["test".to_string()],
        language: LanguageType::En,
        language_category: LanguageCategoryType::En,
        default_pron_index: Some(0),
    }
}

fn test_word(index: i32) -> WordDto {
    WordDto {
        name: format!("bulk-word-{index}"),
        trans: vec![format!("translation-{index}")],
        usphone: String::new(),
        ukphone: String::new(),
        notation: None,
    }
}
