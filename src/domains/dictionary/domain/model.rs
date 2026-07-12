use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "text", rename_all = "lowercase")]
pub enum LanguageType {
    En,
    Romaji,
    Zh,
    Ja,
    Code,
    De,
    Kk,
    Hapin,
    Id,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "text", rename_all = "lowercase")]
pub enum LanguageCategoryType {
    En,
    Ja,
    De,
    Code,
    Kk,
    Id,
}

#[derive(Debug, Clone, FromRow)]
pub struct Dictionary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub tags: Vec<String>,
    pub length: i32,
    pub language: LanguageType,
    pub language_category: LanguageCategoryType,
    pub default_pron_index: Option<i32>,
    pub sort_order: i32,
    pub is_published: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub chapter_count: i32,
}

#[derive(Debug, Clone)]
pub struct NewDictionary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub tags: Vec<String>,
    pub language: LanguageType,
    pub language_category: LanguageCategoryType,
    pub default_pron_index: Option<i32>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Word {
    pub index: i32,
    pub name: String,
    pub trans: Vec<String>,
    pub usphone: String,
    pub ukphone: String,
    pub notation: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewWord {
    pub name: String,
    pub trans: Vec<String>,
    pub usphone: String,
    pub ukphone: String,
    pub notation: Option<String>,
}
