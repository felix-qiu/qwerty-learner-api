use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::domains::{
    dictionary::dto::dictionary_dto::WordDto, record::dto::record_dto::WordRecordDto,
};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(as = ErrorWordData)]
pub struct ErrorWordDataDto {
    #[schema(required = false)]
    pub word: String,
    #[schema(required = false)]
    pub dict: String,
    #[schema(required = false)]
    pub wrong_count: i32,
    /// 与 wrongCount 同义（兼容 TErrorWordData）
    #[schema(required = false)]
    pub error_count: i32,
    #[schema(required = false)]
    pub latest_error_time: i64,
    #[schema(required = false)]
    pub error_letters: BTreeMap<String, i32>,
    #[schema(required = false)]
    pub error_char: Vec<String>,
    #[schema(required = false)]
    pub origin_data: WordDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(required = false, value_type = Vec<WordRecordDto>)]
    pub records: Option<Vec<WordRecordDto>>,
}

#[derive(Debug, Clone, Default, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct ErrorBookListQuery {
    pub dict: Option<String>,
    #[param(default = false)]
    pub include_records: Option<bool>,
    #[param(minimum = 1, default = 1)]
    pub page: Option<i64>,
    #[param(minimum = 1, maximum = 100, default = 20)]
    pub page_size: Option<i64>,
    #[param(default = "wrongCount:asc", example = "wrongCount:desc")]
    pub sort: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorBookListResponse {
    #[schema(required = false)]
    pub items: Vec<ErrorWordDataDto>,
    #[schema(required = false)]
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DictErrorWordsResponse {
    #[schema(required = false)]
    pub items: Vec<ErrorWordDataDto>,
}
