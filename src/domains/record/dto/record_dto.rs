use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
#[schema(description = "字母下标(string) -> 错误按键列表")]
pub struct LetterMistakes(pub BTreeMap<String, Vec<String>>);

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(as = WordRecordCreate)]
pub struct WordRecordCreateDto {
    pub word: String,
    pub dict: String,
    /// 正常章节 >=0；复习 -1；错题练习可为 null。
    pub chapter: Option<i32>,
    /// 相邻字母输入时间差（ms）。
    pub timing: Vec<f64>,
    #[schema(minimum = 0)]
    pub wrong_count: i32,
    pub mistakes: LetterMistakes,
    /// 可选；不传则服务端生成。
    pub time_stamp: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(as = WordRecord)]
pub struct WordRecordDto {
    pub id: i64,
    pub word: String,
    pub dict: String,
    /// 正常章节 >=0；复习 -1；错题练习可为 null。
    pub chapter: Option<i32>,
    /// 相邻字母输入时间差（ms）。
    pub timing: Vec<f64>,
    #[schema(minimum = 0)]
    pub wrong_count: i32,
    pub mistakes: LetterMistakes,
    /// 可选；不传则服务端生成。
    pub time_stamp: i64,
    /// sum(timing)
    #[schema(required = false)]
    pub total_time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WordRecordBatchRequest {
    pub items: Vec<WordRecordCreateDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WordRecordBatchResponse {
    pub ids: Vec<i64>,
    pub accepted: usize,
    pub rejected: usize,
}

#[derive(Debug, Clone, Default, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct WordRecordListQuery {
    pub dict: Option<String>,
    /// 省略时不过滤，`null` 查询 NULL，整数按章节精确过滤。
    #[param(value_type = Option<i32>, nullable = true)]
    pub chapter: Option<String>,
    pub word: Option<String>,
    pub wrong_only: Option<bool>,
    pub from: Option<i64>,
    pub to: Option<i64>,
    #[param(minimum = 1, default = 1)]
    pub page: Option<i64>,
    #[param(minimum = 1, maximum = 100, default = 20)]
    pub page_size: Option<i64>,
    #[param(example = "timeStamp:desc")]
    pub sort: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WordRecordListResponse {
    pub items: Vec<WordRecordDto>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct DeleteWordRecordsQuery {
    pub word: String,
    pub dict: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteWordRecordsResponse {
    pub deleted_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(as = ChapterRecordCreate)]
pub struct ChapterRecordCreateDto {
    pub dict: String,
    pub chapter: Option<i32>,
    /// 章节用时（秒）。
    pub time: i32,
    pub correct_count: i32,
    pub wrong_count: i32,
    pub word_count: i32,
    pub correct_word_indexes: Vec<i32>,
    pub word_number: i32,
    pub word_record_ids: Vec<i64>,
    pub time_stamp: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(as = ChapterRecord)]
pub struct ChapterRecordDto {
    pub id: i64,
    pub wpm: i32,
    pub word_accuracy: i32,
    pub dict: String,
    pub chapter: Option<i32>,
    pub time: i32,
    pub correct_count: i32,
    pub wrong_count: i32,
    pub word_count: i32,
    pub correct_word_indexes: Vec<i32>,
    pub word_number: i32,
    pub word_record_ids: Vec<i64>,
    pub time_stamp: i64,
}

#[derive(Debug, Clone, Default, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct ChapterRecordListQuery {
    pub dict: Option<String>,
    /// 省略时不过滤，`null` 查询 NULL，整数按章节精确过滤。
    #[param(value_type = Option<i32>, nullable = true)]
    pub chapter: Option<String>,
    pub from: Option<i64>,
    pub to: Option<i64>,
    #[param(minimum = 1, default = 1)]
    pub page: Option<i64>,
    #[param(minimum = 1, maximum = 100, default = 20)]
    pub page_size: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChapterRecordListResponse {
    pub items: Vec<ChapterRecordDto>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}
