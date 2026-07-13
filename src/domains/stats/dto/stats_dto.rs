use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ChapterStatsQuery {
    pub dict: String,
    pub chapter: i32,
}

#[derive(Debug, Clone, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct AnalysisStatsQuery {
    /// Unix 秒起始。
    pub from: Option<i64>,
    /// Unix 秒结束。
    pub to: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(as = ChapterStats)]
pub struct ChapterStatsDto {
    #[schema(required = false)]
    pub dict: String,
    #[schema(required = false)]
    pub chapter: i32,
    #[schema(required = false)]
    pub exercise_count: i32,
    #[schema(required = false)]
    pub avg_wrong_word_count: f64,
    #[schema(required = false)]
    pub avg_wrong_input_count: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(as = DictStats)]
pub struct DictStatsDto {
    #[schema(required = false)]
    pub dict: String,
    #[schema(required = false)]
    pub exercised_chapter_count: i32,
    #[schema(required = false)]
    pub chapter_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(as = ActivityDay)]
pub struct ActivityDayDto {
    /// Asia/Shanghai 自然日。
    #[schema(required = false, value_type = String, format = Date)]
    pub date: String,
    #[schema(required = false)]
    pub count: i32,
    #[schema(required = false, minimum = 0, maximum = 4)]
    pub level: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WrongTimeRecordDto {
    #[schema(required = false)]
    pub name: String,
    #[schema(required = false)]
    pub value: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(as = AnalysisStats)]
pub struct AnalysisStatsDto {
    #[schema(required = false)]
    pub is_empty: bool,
    #[schema(required = false)]
    pub exercise_record: Vec<ActivityDayDto>,
    #[schema(required = false)]
    pub word_record: Vec<ActivityDayDto>,
    /// [date, wpm][]
    #[schema(required = false)]
    pub wpm_record: Vec<(String, i32)>,
    #[schema(required = false)]
    pub accuracy_record: Vec<(String, i32)>,
    #[schema(required = false, inline)]
    pub wrong_time_record: Vec<WrongTimeRecordDto>,
}

impl AnalysisStatsDto {
    pub fn empty() -> Self {
        Self {
            is_empty: true,
            exercise_record: Vec::new(),
            word_record: Vec::new(),
            wpm_record: Vec::new(),
            accuracy_record: Vec::new(),
            wrong_time_record: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(as = StatsSummary)]
pub struct StatsSummaryDto {
    #[schema(required = false)]
    pub word_record_count: i32,
    #[schema(required = false)]
    pub chapter_record_count: i32,
    #[schema(required = false)]
    pub total_time_seconds: i32,
    #[schema(required = false, nullable = true)]
    pub first_practice_at: Option<i64>,
}
