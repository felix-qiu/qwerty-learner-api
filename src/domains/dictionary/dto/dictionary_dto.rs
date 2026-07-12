use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::domains::dictionary::domain::model::{
    Dictionary, LanguageCategoryType, LanguageType, Word,
};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub tags: Vec<String>,
    pub length: i32,
    pub chapter_count: i32,
    pub language: LanguageType,
    pub language_category: LanguageCategoryType,
    pub default_pron_index: Option<i32>,
}

impl From<Dictionary> for DictionaryDto {
    fn from(dictionary: Dictionary) -> Self {
        Self {
            id: dictionary.id,
            name: dictionary.name,
            description: dictionary.description,
            category: dictionary.category,
            tags: dictionary.tags,
            length: dictionary.length,
            chapter_count: dictionary.chapter_count,
            language: dictionary.language,
            language_category: dictionary.language_category,
            default_pron_index: dictionary.default_pron_index,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryCreateDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub tags: Vec<String>,
    pub language: LanguageType,
    pub language_category: LanguageCategoryType,
    pub default_pron_index: Option<i32>,
}

#[derive(Debug, Clone, Default, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct DictionaryListQuery {
    pub language_category: Option<LanguageCategoryType>,
    pub category: Option<String>,
    pub tag: Option<String>,
    pub q: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DictionaryListResponse {
    pub items: Vec<DictionaryDto>,
    pub total: i64,
}

#[derive(Debug, Clone, Default, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct WordListQuery {
    #[param(minimum = 0)]
    pub chapter: Option<i32>,
    #[param(minimum = 0)]
    pub offset: Option<i64>,
    #[param(minimum = 1, maximum = 500, default = 20)]
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(as = Word)]
pub struct WordDto {
    pub name: String,
    pub trans: Vec<String>,
    #[serde(default)]
    #[schema(default = String::default)]
    pub usphone: String,
    #[serde(default)]
    #[schema(default = String::default)]
    pub ukphone: String,
    pub notation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WordWithIndexDto {
    pub name: String,
    pub trans: Vec<String>,
    pub usphone: String,
    pub ukphone: String,
    pub notation: Option<String>,
    pub index: i32,
}

impl From<Word> for WordWithIndexDto {
    fn from(word: Word) -> Self {
        Self {
            name: word.name,
            trans: word.trans,
            usphone: word.usphone,
            ukphone: word.ukphone,
            notation: word.notation,
            index: word.index,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WordListResponse {
    pub dict_id: String,
    pub chapter: Option<i32>,
    pub chapter_count: i32,
    pub total: i32,
    pub words: Vec<WordWithIndexDto>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum WordBulkMode {
    Replace,
    Append,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WordBulkRequest {
    pub mode: WordBulkMode,
    pub words: Vec<WordDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WordBulkResponse {
    pub written: i64,
    pub length: i32,
    pub chapter_count: i32,
}
