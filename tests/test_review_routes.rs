use axum::{
    body::Body,
    http::{header::CONTENT_TYPE, Method, Request, StatusCode},
};
use clean_axum_demo::{
    app::create_router,
    common::{
        bootstrap::build_app_state, config::Config, error::ErrorResponse, jwt::make_access_token,
    },
    domains::{
        dictionary::dto::dictionary_dto::WordDto,
        review::{
            dto::review_dto::{
                LatestReviewResponse, ReviewCreateDto, ReviewErrorDataDto, ReviewListResponse,
                ReviewRecordDto, ReviewUpdateDto,
            },
            ReviewApiDoc,
        },
    },
};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
use utoipa::OpenApi;

mod test_helpers;

use test_helpers::{
    deserialize_json_body, request_with_auth, request_with_auth_and_body, TEST_USER_ID,
};

#[tokio::test]
async fn test_review_contract_and_rejections() {
    dotenvy::from_filename(".env.test").expect("Failed to load .env.test");
    let config = Config::from_env().unwrap();
    let pool = PgPoolOptions::new()
        .connect_lazy(&config.database_url)
        .unwrap();
    let router = create_router(build_app_state(pool, config));

    let openapi = ReviewApiDoc::openapi();
    assert_eq!(openapi.servers.as_ref().unwrap()[0].url, "/api/v1");
    assert!(openapi
        .components
        .as_ref()
        .unwrap()
        .security_schemes
        .contains_key("bearerAuth"));
    let openapi_json = serde_json::to_value(&openapi).unwrap();
    for (pointer, operation_id) in [
        ("/paths/~1reviews/get/operationId", "listReviews"),
        ("/paths/~1reviews/post/operationId", "createReview"),
        (
            "/paths/~1reviews~1latest/get/operationId",
            "getLatestReview",
        ),
        (
            "/paths/~1reviews~1{reviewId}/patch/operationId",
            "updateReview",
        ),
    ] {
        assert_eq!(openapi_json.pointer(pointer).unwrap(), operation_id);
    }
    assert_eq!(
        openapi_json
            .pointer("/components/schemas/ReviewRecord/properties/index/minimum")
            .unwrap(),
        0
    );
    assert_eq!(
        openapi_json
            .pointer("/components/schemas/ReviewUpdate/properties/words/type/1")
            .unwrap(),
        "null"
    );
    assert_eq!(
        openapi_json
            .pointer("/components/schemas/ReviewUpdate/properties/words/items/$ref")
            .unwrap(),
        "#/components/schemas/Word"
    );
    assert_eq!(
        openapi_json
            .pointer("/components/schemas/ReviewCreate/properties/errorData/type")
            .unwrap(),
        "array"
    );
    assert_eq!(
        openapi_json
            .pointer("/components/schemas/ReviewUpdate/properties/index/type")
            .unwrap(),
        "integer"
    );
    assert_eq!(
        openapi_json
            .pointer("/components/schemas/ReviewUpdate/properties/isFinished/type")
            .unwrap(),
        "boolean"
    );
    assert!(openapi_json
        .pointer("/components/schemas/ReviewRecord/required")
        .is_none());
    assert!(openapi_json
        .pointer("/components/schemas/ReviewListResponse/required")
        .is_none());
    assert_eq!(
        openapi_json
            .pointer("/components/schemas/Word/properties/usphone/default")
            .unwrap(),
        ""
    );
    assert_eq!(
        openapi_json
            .pointer("/paths/~1reviews/post/responses/201/description")
            .unwrap(),
        "Created"
    );

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/reviews")
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
                .uri("/api/v1/reviews")
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
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/reviews")
                .header(CONTENT_TYPE, "application/json")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::from(r#"{"dict":"cet4","errorData":null}"#))
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
                .uri("/api/v1/reviews?page=9223372036854775807&pageSize=100")
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
                .method(Method::PATCH)
                .uri("/api/v1/reviews/not-an-integer")
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
}

#[tokio::test]
async fn test_review_lifecycle_and_latest_semantics() {
    let dict = format!("reviews-{}", uuid::Uuid::new_v4());
    let payload = ReviewCreateDto {
        dict: dict.clone(),
        error_data: Some(vec![
            review_error("frequent", 5, 300),
            review_error("old", 1, 100),
            review_error("middle", 3, 200),
        ]),
    };

    let response = request_with_auth_and_body(Method::POST, "/api/v1/reviews", &payload).await;
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::CREATED);
    let older: ReviewRecordDto = deserialize_json_body(body).await.unwrap();
    assert_eq!(older.index, 0);
    assert!(!older.is_finished);
    assert_eq!(older.words[0].name, "old");

    let response = request_with_auth_and_body(Method::POST, "/api/v1/reviews", &payload).await;
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::CREATED);
    let latest_created: ReviewRecordDto = deserialize_json_body(body).await.unwrap();
    assert!(latest_created.id > older.id);

    let latest_uri = format!("/api/v1/reviews/latest?dict={dict}");
    let response = request_with_auth(Method::GET, &latest_uri).await;
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::OK);
    let latest: LatestReviewResponse = deserialize_json_body(body).await.unwrap();
    assert_eq!(latest.record.unwrap().id, latest_created.id);

    let update = ReviewUpdateDto {
        index: Some(2),
        is_finished: Some(true),
        words: None,
    };
    let update_uri = format!("/api/v1/reviews/{}", latest_created.id);
    let response = request_with_auth_and_body(Method::PATCH, &update_uri, &update).await;
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::OK);
    let updated: ReviewRecordDto = deserialize_json_body(body).await.unwrap();
    assert_eq!(updated.index, 2);
    assert!(updated.is_finished);
    assert_eq!(updated.words.len(), 3);

    let response = request_with_auth(Method::GET, &latest_uri).await;
    let (_, body) = response.into_parts();
    let latest: LatestReviewResponse = deserialize_json_body(body).await.unwrap();
    assert!(latest.record.is_none());

    let list_uri = format!("/api/v1/reviews?dict={dict}&page=1&pageSize=20");
    let response = request_with_auth(Method::GET, &list_uri).await;
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::OK);
    let list: ReviewListResponse = deserialize_json_body(body).await.unwrap();
    assert_eq!(list.total, 2);
    assert_eq!(list.items[0].id, latest_created.id);
    assert_eq!(list.items[1].id, older.id);
}

fn review_error(name: &str, error_count: i32, latest_error_time: i64) -> ReviewErrorDataDto {
    ReviewErrorDataDto {
        word: name.into(),
        error_count,
        latest_error_time,
        origin_data: WordDto {
            name: name.into(),
            trans: vec![name.into()],
            usphone: String::new(),
            ukphone: String::new(),
            notation: None,
        },
    }
}
