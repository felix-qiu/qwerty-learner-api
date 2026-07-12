use std::collections::BTreeMap;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use clean_axum_demo::{
    app::create_router,
    common::{
        bootstrap::build_app_state, config::Config, error::ErrorResponse, jwt::make_access_token,
    },
    domains::{
        error_book::{
            dto::error_book_dto::{DictErrorWordsResponse, ErrorBookListResponse},
            ErrorBookApiDoc,
        },
        record::dto::record_dto::{LetterMistakes, WordRecordCreateDto},
    },
};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
use utoipa::OpenApi;

mod test_helpers;

use test_helpers::{
    deserialize_json_body, request_with_auth, request_with_auth_and_body, setup_test_db,
    TEST_USER_ID,
};

#[tokio::test]
async fn test_error_book_contract_and_rejections() {
    dotenvy::from_filename(".env.test").expect("Failed to load .env.test");
    let config = Config::from_env().unwrap();
    let pool = PgPoolOptions::new()
        .connect_lazy(&config.database_url)
        .unwrap();
    let router = create_router(build_app_state(pool, config));

    let openapi = ErrorBookApiDoc::openapi();
    assert_eq!(openapi.servers.as_ref().unwrap()[0].url, "/api/v1");
    assert!(openapi
        .components
        .as_ref()
        .unwrap()
        .security_schemes
        .contains_key("bearerAuth"));
    let openapi_json = serde_json::to_value(&openapi).unwrap();
    assert_eq!(
        openapi_json
            .pointer("/paths/~1error-book/get/operationId")
            .unwrap(),
        "listErrorBook"
    );
    assert_eq!(
        openapi_json
            .pointer("/paths/~1dictionaries~1{dictId}~1error-words/get/operationId")
            .unwrap(),
        "getDictErrorWords"
    );
    assert_eq!(
        openapi_json
            .pointer("/components/schemas/ErrorWordData/properties/records/type")
            .unwrap(),
        "array"
    );
    assert_eq!(
        openapi_json
            .pointer("/components/schemas/ErrorWordData/properties/records/items/$ref")
            .unwrap(),
        "#/components/schemas/WordRecord"
    );
    assert_eq!(
        openapi_json
            .pointer(
                "/components/schemas/ErrorWordData/properties/errorLetters/additionalProperties/type",
            )
            .unwrap(),
        "integer"
    );
    assert_eq!(
        openapi_json
            .pointer("/components/schemas/WordRecord/properties/wrongCount/minimum")
            .unwrap(),
        0
    );
    assert!(!openapi_json
        .pointer("/components/schemas/WordRecord/required")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .any(|field| field == "totalTime"));
    assert_eq!(
        query_parameter(&openapi_json, "includeRecords")
            .pointer("/schema/default")
            .unwrap(),
        false
    );
    assert_eq!(
        query_parameter(&openapi_json, "pageSize")
            .pointer("/schema/default")
            .unwrap(),
        20
    );
    assert_eq!(
        query_parameter(&openapi_json, "sort")
            .pointer("/schema/default")
            .unwrap(),
        "wrongCount:asc"
    );

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/error-book")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let token = make_access_token(TEST_USER_ID).unwrap();
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/error-book?sort=timeStamp:desc")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::UNPROCESSABLE_ENTITY);
    let error: ErrorResponse = deserialize_json_body(body).await.unwrap();
    assert_eq!(error.code, "VALIDATION_ERROR");

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/error-book?page=9223372036854775807&pageSize=100")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_error_book_aggregation_and_dictionary_view() {
    let pool = setup_test_db().await.unwrap();
    cleanup_word_records(&pool).await;

    for payload in [
        word_record(2, 1_720_000_001, [("0", vec!["x", "y"]), ("2", vec!["z"])]),
        word_record(
            3,
            1_720_000_002,
            [("0", vec!["q"]), ("1", vec!["w", "e", "r"])],
        ),
    ] {
        let response =
            request_with_auth_and_body(Method::POST, "/api/v1/records/words", &payload).await;
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    let response = request_with_auth(
        Method::GET,
        "/api/v1/error-book?dict=apitest-dict&page=1&pageSize=20&sort=wrongCount:asc",
    )
    .await;
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::OK);
    let list: ErrorBookListResponse = deserialize_json_body(body).await.unwrap();
    assert_eq!(list.total, 1);
    let item = &list.items[0];
    assert_eq!(item.word, "word24");
    assert_eq!(item.wrong_count, 5);
    assert_eq!(item.error_count, 5);
    assert_eq!(item.latest_error_time, 1_720_000_002);
    assert_eq!(item.error_letters.get("0"), Some(&3));
    assert_eq!(item.error_letters.get("1"), Some(&3));
    assert_eq!(item.error_letters.get("2"), Some(&1));
    assert_eq!(item.error_char, vec!["w", "o", "r"]);
    assert_eq!(item.origin_data.trans, vec!["translation 24"]);
    assert!(item.records.is_none());

    let response = request_with_auth(
        Method::GET,
        "/api/v1/error-book?dict=apitest-dict&includeRecords=true",
    )
    .await;
    let (_, body) = response.into_parts();
    let list: ErrorBookListResponse = deserialize_json_body(body).await.unwrap();
    let records = list.items[0].records.as_ref().unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].time_stamp, 1_720_000_002);

    let response =
        request_with_auth(Method::GET, "/api/v1/dictionaries/apitest-dict/error-words").await;
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::OK);
    let dictionary: DictErrorWordsResponse = deserialize_json_body(body).await.unwrap();
    assert_eq!(dictionary.items.len(), 1);
    assert!(dictionary.items[0].records.is_none());

    let response = request_with_auth(
        Method::GET,
        "/api/v1/dictionaries/missing-dictionary/error-words",
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    cleanup_word_records(&pool).await;
}

fn query_parameter<'a>(openapi: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    openapi
        .pointer("/paths/~1error-book/get/parameters")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|parameter| parameter.get("name").and_then(|name| name.as_str()) == Some(name))
        .unwrap()
}

fn word_record<const N: usize>(
    wrong_count: i32,
    time_stamp: i64,
    mistakes: [(&str, Vec<&str>); N],
) -> WordRecordCreateDto {
    WordRecordCreateDto {
        word: "word24".into(),
        dict: "apitest-dict".into(),
        chapter: Some(1),
        timing: vec![20.0, 30.0],
        wrong_count,
        mistakes: LetterMistakes(BTreeMap::from_iter(mistakes.into_iter().map(
            |(index, values)| {
                (
                    index.to_string(),
                    values.into_iter().map(str::to_string).collect(),
                )
            },
        ))),
        time_stamp: Some(time_stamp),
    }
}

async fn cleanup_word_records(pool: &sqlx::PgPool) {
    sqlx::query(
        "DELETE FROM word_records WHERE user_id = $1 AND dict = 'apitest-dict' AND word = 'word24'",
    )
    .bind(TEST_USER_ID)
    .execute(pool)
    .await
    .unwrap();
}
