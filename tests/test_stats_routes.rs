use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use clean_axum_demo::{
    app::create_router,
    common::{bootstrap::build_app_state, config::Config},
    domains::stats::{
        dto::stats_dto::{AnalysisStatsDto, ChapterStatsDto, DictStatsDto, StatsSummaryDto},
        StatsApiDoc,
    },
};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
use utoipa::OpenApi;

mod test_helpers;

use test_helpers::{deserialize_json_body, request_with_auth, setup_test_db, TEST_USER_ID};

#[tokio::test]
async fn test_stats_contract_and_authentication() {
    dotenvy::from_filename(".env.test").expect("Failed to load .env.test");
    let config = Config::from_env().unwrap();
    let pool = PgPoolOptions::new()
        .connect_lazy(&config.database_url)
        .unwrap();
    let router = create_router(build_app_state(pool, config));

    let openapi = StatsApiDoc::openapi();
    assert_eq!(openapi.servers.as_ref().unwrap()[0].url, "/api/v1");
    assert!(openapi
        .components
        .as_ref()
        .unwrap()
        .security_schemes
        .contains_key("bearerAuth"));
    let openapi_json = serde_json::to_value(&openapi).unwrap();
    for (pointer, operation_id) in [
        (
            "/paths/~1stats~1chapters/get/operationId",
            "getChapterStats",
        ),
        (
            "/paths/~1stats~1dictionaries~1{dictId}/get/operationId",
            "getDictStats",
        ),
        (
            "/paths/~1stats~1analysis/get/operationId",
            "getAnalysisStats",
        ),
        ("/paths/~1stats~1summary/get/operationId", "getStatsSummary"),
    ] {
        assert_eq!(openapi_json.pointer(pointer).unwrap(), operation_id);
    }
    for path in [
        "/paths/~1stats~1chapters/get",
        "/paths/~1stats~1dictionaries~1{dictId}/get",
        "/paths/~1stats~1analysis/get",
        "/paths/~1stats~1summary/get",
    ] {
        assert_eq!(
            openapi_json
                .pointer(&format!("{path}/security/0/bearerAuth"))
                .unwrap(),
            &serde_json::json!([])
        );
    }
    for (path, schema) in [
        ("~1stats~1chapters", "ChapterStats"),
        ("~1stats~1dictionaries~1{dictId}", "DictStats"),
        ("~1stats~1analysis", "AnalysisStats"),
        ("~1stats~1summary", "StatsSummary"),
    ] {
        assert_eq!(
            openapi_json
                .pointer(&format!(
                    "/paths/{path}/get/responses/200/content/application~1json/schema/$ref"
                ))
                .unwrap(),
            &format!("#/components/schemas/{schema}")
        );
    }
    assert_eq!(
        query_parameter(&openapi_json, "/paths/~1stats~1chapters/get", "dict")
            .get("required")
            .unwrap(),
        true
    );
    assert_eq!(
        query_parameter(&openapi_json, "/paths/~1stats~1chapters/get", "chapter")
            .get("required")
            .unwrap(),
        true
    );
    for name in ["from", "to"] {
        assert_eq!(
            query_parameter(&openapi_json, "/paths/~1stats~1analysis/get", name)
                .get("required")
                .unwrap(),
            false
        );
    }
    assert_eq!(
        openapi_json
            .pointer("/components/schemas/ActivityDay/properties/date/format")
            .unwrap(),
        "date"
    );
    assert_eq!(
        openapi_json
            .pointer("/components/schemas/ActivityDay/properties/level/minimum")
            .unwrap(),
        0
    );
    assert_eq!(
        openapi_json
            .pointer("/components/schemas/ActivityDay/properties/level/maximum")
            .unwrap(),
        4
    );
    assert!(openapi_json
        .pointer("/components/schemas/ChapterStats/required")
        .is_none());
    assert!(openapi_json
        .pointer("/components/schemas/AnalysisStats/required")
        .is_none());
    assert_eq!(
        openapi_json
            .pointer("/components/schemas/AnalysisStats/properties/wpmRecord/items/minItems")
            .unwrap(),
        2
    );
    assert_eq!(
        openapi_json
            .pointer("/components/schemas/AnalysisStats/properties/wpmRecord/items/maxItems")
            .unwrap(),
        2
    );
    assert_eq!(
        openapi_json
            .pointer("/components/schemas/AnalysisStats/properties/wpmRecord/items/items")
            .unwrap(),
        &serde_json::json!({})
    );
    assert!(openapi_json
        .pointer("/components/schemas/AnalysisStats/properties/wpmRecord/items/prefixItems")
        .is_none());
    assert_eq!(
        openapi_json
            .pointer("/components/schemas/AnalysisStats/properties/accuracyRecord/items/minItems")
            .unwrap(),
        2
    );
    assert_eq!(
        openapi_json
            .pointer("/components/schemas/AnalysisStats/properties/accuracyRecord/items/maxItems")
            .unwrap(),
        2
    );
    assert_eq!(
        openapi_json
            .pointer("/components/schemas/AnalysisStats/properties/accuracyRecord/items/items")
            .unwrap(),
        &serde_json::json!({})
    );
    assert_eq!(
        openapi_json
            .pointer("/components/schemas/AnalysisStats/properties/wrongTimeRecord/items/properties/name/type")
            .unwrap(),
        "string"
    );
    assert_eq!(
        openapi_json
            .pointer("/components/schemas/AnalysisStats/properties/wrongTimeRecord/items/properties/value/type")
            .unwrap(),
        "integer"
    );
    assert!(openapi_json
        .pointer("/components/schemas/WrongTimeRecordDto")
        .is_none());
    let first_practice_at = openapi_json
        .pointer("/components/schemas/StatsSummary/properties/firstPracticeAt")
        .unwrap();
    assert!(
        first_practice_at.get("nullable") == Some(&serde_json::json!(true))
            || first_practice_at.to_string().contains("\"null\"")
    );

    for uri in [
        "/api/v1/stats/chapters?dict=apitest-dict&chapter=0",
        "/api/v1/stats/dictionaries/apitest-dict",
        "/api/v1/stats/analysis",
        "/api/v1/stats/summary",
    ] {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}

fn query_parameter<'a>(
    openapi: &'a serde_json::Value,
    path: &str,
    name: &str,
) -> &'a serde_json::Value {
    openapi
        .pointer(&format!("{path}/parameters"))
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|parameter| parameter.get("name").and_then(|name| name.as_str()) == Some(name))
        .unwrap()
}

#[tokio::test]
async fn test_stats_aggregation_uses_new_schema_rules() {
    let pool = setup_test_db().await.unwrap();
    cleanup_stats_records(&pool).await;

    for (chapter, wrong_count, correct_word_indexes, time_stamp) in [
        (Some(1), 3, (0..18).collect::<Vec<_>>(), 1_751_382_000),
        (Some(1), 5, (0..16).collect::<Vec<_>>(), 1_751_382_001),
        (Some(-1), 1, vec![0], 1_751_382_002),
    ] {
        sqlx::query(
            r#"
            INSERT INTO chapter_records
                (user_id, dict, chapter, time_stamp, time_seconds, correct_count,
                 wrong_count, word_count, correct_word_indexes, word_number, word_record_ids)
            VALUES ($1, 'apitest-dict', $2, $3, 60, 10, $4, 20, $5, 20, '{}')
            "#,
        )
        .bind(TEST_USER_ID)
        .bind(chapter)
        .bind(time_stamp)
        .bind(wrong_count)
        .bind(correct_word_indexes)
        .execute(&pool)
        .await
        .unwrap();
    }

    for (word, timing, wrong_count, mistakes, time_stamp) in [
        (
            "ab",
            vec![60_000.0],
            1,
            serde_json::json!({"0": ["a"]}),
            1_751_385_540,
        ),
        (
            "bb",
            vec![30_000.0],
            1,
            serde_json::json!({"0": ["e"]}),
            1_751_385_660,
        ),
        (
            "bb",
            vec![30_000.0],
            0,
            serde_json::json!({}),
            1_751_385_720,
        ),
    ] {
        sqlx::query(
            r#"
            INSERT INTO word_records
                (user_id, word, dict, chapter, timing, wrong_count, mistakes, time_stamp)
            VALUES ($1, $2, 'apitest-dict', 1, $3, $4, $5, $6)
            "#,
        )
        .bind(TEST_USER_ID)
        .bind(word)
        .bind(timing)
        .bind(wrong_count)
        .bind(mistakes)
        .bind(time_stamp)
        .execute(&pool)
        .await
        .unwrap();
    }

    let response = request_with_auth(
        Method::GET,
        "/api/v1/stats/chapters?dict=apitest-dict&chapter=1",
    )
    .await;
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::OK);
    let chapter: ChapterStatsDto = deserialize_json_body(body).await.unwrap();
    assert_eq!(chapter.exercise_count, 2);
    assert_eq!(chapter.avg_wrong_word_count, 3.0);
    assert_eq!(chapter.avg_wrong_input_count, 4.0);

    let response = request_with_auth(
        Method::GET,
        "/api/v1/stats/chapters?dict=apitest-dict&chapter=2",
    )
    .await;
    let (_, body) = response.into_parts();
    let empty_chapter: ChapterStatsDto = deserialize_json_body(body).await.unwrap();
    assert_eq!(empty_chapter.exercise_count, 0);
    assert_eq!(empty_chapter.avg_wrong_word_count, 0.0);
    assert_eq!(empty_chapter.avg_wrong_input_count, 0.0);

    let response = request_with_auth(Method::GET, "/api/v1/stats/dictionaries/apitest-dict").await;
    let (_, body) = response.into_parts();
    let dictionary: DictStatsDto = deserialize_json_body(body).await.unwrap();
    assert_eq!(dictionary.exercised_chapter_count, 1);
    assert_eq!(dictionary.chapter_count, 2);

    let response = request_with_auth(
        Method::GET,
        "/api/v1/stats/analysis?from=1751385540&to=1751385720",
    )
    .await;
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::OK);
    let analysis: AnalysisStatsDto = deserialize_json_body(body).await.unwrap();
    assert!(!analysis.is_empty);
    assert_eq!(analysis.exercise_record.len(), 2);
    assert_eq!(analysis.exercise_record[0].date, "2025-07-01");
    assert_eq!(analysis.exercise_record[0].count, 1);
    assert_eq!(analysis.exercise_record[1].date, "2025-07-02");
    assert_eq!(analysis.exercise_record[1].count, 2);
    assert_eq!(analysis.word_record[1].count, 1);
    assert_eq!(
        analysis.wpm_record,
        vec![("2025-07-01".into(), 1), ("2025-07-02".into(), 2)]
    );
    assert_eq!(
        analysis.accuracy_record,
        vec![("2025-07-01".into(), 67), ("2025-07-02".into(), 80)]
    );
    assert_eq!(analysis.wrong_time_record[0].name, "A");
    assert_eq!(analysis.wrong_time_record[1].name, "E");

    let response = request_with_auth(Method::GET, "/api/v1/stats/summary").await;
    let (parts, body) = response.into_parts();
    assert_eq!(parts.status, StatusCode::OK);
    let summary: StatsSummaryDto = deserialize_json_body(body).await.unwrap();
    assert!(summary.word_record_count >= 3);
    assert!(summary.chapter_record_count >= 3);
    assert!(summary.total_time_seconds >= 180);
    assert!(summary.first_practice_at.is_some());

    cleanup_stats_records(&pool).await;
}

async fn cleanup_stats_records(pool: &sqlx::PgPool) {
    sqlx::query("DELETE FROM word_records WHERE user_id = $1 AND dict = 'apitest-dict' AND time_stamp BETWEEN 1751385540 AND 1751385720")
        .bind(TEST_USER_ID)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM chapter_records WHERE user_id = $1 AND dict = 'apitest-dict' AND time_stamp BETWEEN 1751382000 AND 1751382002")
        .bind(TEST_USER_ID)
        .execute(pool)
        .await
        .unwrap();
}
