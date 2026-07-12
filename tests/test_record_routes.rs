use std::collections::BTreeMap;

use axum::{
    body::Body,
    http::{header::CONTENT_TYPE, Method, Request, StatusCode},
};
use clean_axum_demo::{
    app::create_router,
    common::{
        bootstrap::build_app_state, config::Config, error::ErrorResponse, jwt::make_access_token,
    },
    domains::record::{
        dto::record_dto::{
            ChapterRecordCreateDto, ChapterRecordDto, ChapterRecordListResponse,
            DeleteWordRecordsResponse, LetterMistakes, WordRecordBatchRequest,
            WordRecordBatchResponse, WordRecordCreateDto, WordRecordDto, WordRecordListResponse,
        },
        RecordApiDoc,
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
async fn test_record_contract_and_rejections() {
    dotenvy::from_filename(".env.test").expect("Failed to load .env.test");
    let config = Config::from_env().unwrap();
    let pool = PgPoolOptions::new()
        .connect_lazy(&config.database_url)
        .unwrap();
    let state = build_app_state(pool, config);
    let router = create_router(state);

    let openapi = RecordApiDoc::openapi();
    assert_eq!(openapi.servers.as_ref().unwrap()[0].url, "/api/v1");
    assert!(openapi
        .components
        .as_ref()
        .unwrap()
        .security_schemes
        .contains_key("bearerAuth"));
    let schemas = &openapi.components.as_ref().unwrap().schemas;
    for schema in [
        "LetterMistakes",
        "WordRecordCreate",
        "WordRecord",
        "ChapterRecordCreate",
        "ChapterRecord",
    ] {
        assert!(schemas.contains_key(schema));
    }
    let openapi_json = serde_json::to_value(&openapi).unwrap();
    for (pointer, operation_id) in [
        (
            "/paths/~1records~1words/post/operationId",
            "createWordRecord",
        ),
        ("/paths/~1records~1words/get/operationId", "listWordRecords"),
        (
            "/paths/~1records~1words/delete/operationId",
            "deleteWordRecords",
        ),
        (
            "/paths/~1records~1words:batch/post/operationId",
            "batchCreateWordRecords",
        ),
        (
            "/paths/~1records~1chapters/post/operationId",
            "createChapterRecord",
        ),
        (
            "/paths/~1records~1chapters/get/operationId",
            "listChapterRecords",
        ),
    ] {
        assert_eq!(openapi_json.pointer(pointer).unwrap(), operation_id);
    }
    assert_eq!(
        openapi_json
            .pointer("/components/schemas/WordRecordCreate/properties/wrongCount/minimum")
            .unwrap(),
        0
    );

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/records/words")
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
                .method(Method::POST)
                .uri("/api/v1/records/words")
                .header(CONTENT_TYPE, "application/json")
                .header("Authorization", format!("Bearer {token}"))
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
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/records/words?sort=id:desc")
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
}

#[tokio::test]
async fn test_word_record_lifecycle_and_partial_batch() {
    let unique = uuid::Uuid::new_v4();
    let dict = format!("records-{unique}");
    let word = format!("word-{unique}");
    let payload = word_payload(word.clone(), dict.clone(), 1);

    let response =
        request_with_auth_and_body(Method::POST, "/api/v1/records/words", &payload).await;
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::CREATED);
    let created: WordRecordDto = deserialize_json_body(body).await.unwrap();
    assert_eq!(created.total_time, 60.0);
    assert_eq!(created.wrong_count, 1);

    let batch = WordRecordBatchRequest {
        items: vec![
            word_payload(format!("batch-{unique}"), dict.clone(), 2),
            WordRecordCreateDto {
                word: String::new(),
                ..word_payload(format!("rejected-{unique}"), dict.clone(), 0)
            },
        ],
    };
    let response =
        request_with_auth_and_body(Method::POST, "/api/v1/records/words:batch", &batch).await;
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::CREATED);
    let batch_result: WordRecordBatchResponse = deserialize_json_body(body).await.unwrap();
    assert_eq!(batch_result.accepted, 1);
    assert_eq!(batch_result.rejected, 1);
    assert_eq!(batch_result.ids.len(), 1);

    let uri = format!(
        "/api/v1/records/words?dict={dict}&wrongOnly=true&page=1&pageSize=20&sort=timeStamp:asc"
    );
    let response = request_with_auth(Method::GET, &uri).await;
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::OK);
    let list: WordRecordListResponse = deserialize_json_body(body).await.unwrap();
    assert_eq!(list.total, 2);
    assert_eq!(list.page, 1);
    assert_eq!(list.page_size, 20);

    let delete_uri = format!("/api/v1/records/words?word={word}&dict={dict}");
    let response = request_with_auth(Method::DELETE, &delete_uri).await;
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::OK);
    let deleted: DeleteWordRecordsResponse = deserialize_json_body(body).await.unwrap();
    assert_eq!(deleted.deleted_count, 1);
}

#[tokio::test]
async fn test_disabled_user_cannot_access_records() {
    let pool = setup_test_db().await.unwrap();
    let user_id = format!("usr_{}", uuid::Uuid::new_v4());
    sqlx::query(
        r#"
        INSERT INTO users (id, email, password_hash, role, status)
        VALUES ($1, $2, 'disabled-test-hash', 'user', 'disabled')
        "#,
    )
    .bind(&user_id)
    .bind(format!("disabled-{}@example.com", uuid::Uuid::new_v4()))
    .execute(&pool)
    .await
    .unwrap();

    let config = Config::from_env().unwrap();
    let router = create_router(build_app_state(pool.clone(), config));
    let token = make_access_token(&user_id).unwrap();
    let response = router
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/records/words")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_chapter_record_create_and_list() {
    let dict = format!("chapters-{}", uuid::Uuid::new_v4());
    let payload = ChapterRecordCreateDto {
        dict: dict.clone(),
        chapter: Some(0),
        time: 185,
        correct_count: 312,
        wrong_count: 8,
        word_count: 22,
        correct_word_indexes: vec![0, 1, 3, 4, 5],
        word_number: 20,
        word_record_ids: vec![],
        time_stamp: Some(1_719_900_300),
    };

    let response =
        request_with_auth_and_body(Method::POST, "/api/v1/records/chapters", &payload).await;
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::CREATED);
    let created: ChapterRecordDto = deserialize_json_body(body).await.unwrap();
    assert_eq!(created.wpm, 7);
    assert_eq!(created.word_accuracy, 25);

    let null_chapter_payload = ChapterRecordCreateDto {
        chapter: None,
        time_stamp: Some(1_719_900_301),
        ..payload.clone()
    };
    let response = request_with_auth_and_body(
        Method::POST,
        "/api/v1/records/chapters",
        &null_chapter_payload,
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let uri = format!("/api/v1/records/chapters?dict={dict}&chapter=0&page=1&pageSize=20");
    let response = request_with_auth(Method::GET, &uri).await;
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::OK);
    let list: ChapterRecordListResponse = deserialize_json_body(body).await.unwrap();
    assert_eq!(list.total, 1);
    assert_eq!(list.items[0].id, created.id);

    let uri = format!("/api/v1/records/chapters?dict={dict}&chapter=null");
    let response = request_with_auth(Method::GET, &uri).await;
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::OK);
    let list: ChapterRecordListResponse = deserialize_json_body(body).await.unwrap();
    assert_eq!(list.total, 1);
    assert!(list.items[0].chapter.is_none());
}

fn word_payload(word: String, dict: String, wrong_count: i32) -> WordRecordCreateDto {
    let mut mistakes = BTreeMap::new();
    mistakes.insert("0".to_string(), vec!["x".to_string()]);
    WordRecordCreateDto {
        word,
        dict,
        chapter: Some(0),
        timing: vec![10.0, 20.0, 30.0],
        wrong_count,
        mistakes: LetterMistakes(mistakes),
        time_stamp: None,
    }
}
