use serde_json::Value;
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct WordRecord {
    pub id: i64,
    pub word: String,
    pub dict: String,
    pub chapter: Option<i32>,
    pub timing: Vec<f64>,
    pub wrong_count: i32,
    pub mistakes: Value,
    pub time_stamp: i64,
}

#[derive(Debug, Clone)]
pub struct NewWordRecord {
    pub user_id: String,
    pub word: String,
    pub dict: String,
    pub chapter: Option<i32>,
    pub timing: Vec<f64>,
    pub wrong_count: i32,
    pub mistakes: Value,
    pub time_stamp: i64,
}

#[derive(Debug, Clone, FromRow)]
pub struct ChapterRecord {
    pub id: i64,
    pub dict: String,
    pub chapter: Option<i32>,
    pub time_stamp: i64,
    pub time_seconds: i32,
    pub correct_count: i32,
    pub wrong_count: i32,
    pub word_count: i32,
    pub correct_word_indexes: Vec<i32>,
    pub word_number: i32,
    pub word_record_ids: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct NewChapterRecord {
    pub user_id: String,
    pub dict: String,
    pub chapter: Option<i32>,
    pub time_stamp: i64,
    pub time_seconds: i32,
    pub correct_count: i32,
    pub wrong_count: i32,
    pub word_count: i32,
    pub correct_word_indexes: Vec<i32>,
    pub word_number: i32,
    pub word_record_ids: Vec<i64>,
}
