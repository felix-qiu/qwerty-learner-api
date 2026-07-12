use serde::{Deserialize, Deserializer, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::domains::dictionary::dto::dictionary_dto::WordDto;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(as = ReviewRecord)]
pub struct ReviewRecordDto {
    #[schema(required = false)]
    pub id: i64,
    #[schema(required = false)]
    pub dict: String,
    #[schema(required = false, minimum = 0)]
    pub index: i32,
    #[schema(required = false)]
    pub create_time: i64,
    #[schema(required = false)]
    pub is_finished: bool,
    #[schema(required = false)]
    pub words: Vec<WordDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReviewErrorDataDto {
    pub word: String,
    #[schema(minimum = 0)]
    pub error_count: i32,
    pub latest_error_time: i64,
    pub origin_data: WordDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(as = ReviewCreate)]
pub struct ReviewCreateDto {
    pub dict: String,
    /// 可选；不传则服务端自行聚合错题。
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    #[schema(value_type = Vec<ReviewErrorDataDto>)]
    pub error_data: Option<Vec<ReviewErrorDataDto>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(as = ReviewUpdate)]
pub struct ReviewUpdateDto {
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    #[schema(value_type = i32, minimum = 0)]
    pub index: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    #[schema(value_type = bool)]
    pub is_finished: Option<bool>,
    /// null 或省略均不修改；空数组用于清空。
    #[schema(nullable = true)]
    pub words: Option<Vec<WordDto>>,
}

#[derive(Debug, Clone, Default, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct ReviewListQuery {
    pub dict: Option<String>,
    #[param(minimum = 1, default = 1)]
    pub page: Option<i64>,
    #[param(minimum = 1, maximum = 100, default = 20)]
    pub page_size: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReviewListResponse {
    #[schema(required = false)]
    pub items: Vec<ReviewRecordDto>,
    #[schema(required = false)]
    pub total: i64,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct LatestReviewQuery {
    pub dict: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LatestReviewResponse {
    #[schema(nullable = true)]
    pub record: Option<ReviewRecordDto>,
}

fn deserialize_optional_non_null<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}
