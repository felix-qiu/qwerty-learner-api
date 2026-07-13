use serde_json::Value;
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct ChapterStatsRow {
    pub exercise_count: i32,
    pub avg_wrong_word_count: f64,
    pub avg_wrong_input_count: f64,
}

#[derive(Debug, Clone, FromRow)]
pub struct StatsSummaryRow {
    pub word_record_count: i32,
    pub chapter_record_count: i32,
    pub total_time_seconds: i32,
    pub first_practice_at: Option<i64>,
}

#[derive(Debug, Clone, FromRow)]
pub struct AnalysisWordRecord {
    pub word: String,
    pub timing: Vec<f64>,
    pub wrong_count: i32,
    pub mistakes: Value,
    pub time_stamp: i64,
}
