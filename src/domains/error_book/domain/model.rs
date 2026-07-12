use serde_json::Value;
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct ErrorWordGroup {
    pub word: String,
    pub dict: String,
    pub wrong_count: i32,
    pub latest_error_time: i64,
    pub trans: Vec<String>,
    pub usphone: String,
    pub ukphone: String,
    pub notation: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ErrorWordRecord {
    pub id: i64,
    pub word: String,
    pub dict: String,
    pub chapter: Option<i32>,
    pub timing: Vec<f64>,
    pub wrong_count: i32,
    pub mistakes: Value,
    pub time_stamp: i64,
}
