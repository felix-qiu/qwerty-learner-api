use serde_json::Value;
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct ReviewRecord {
    pub id: i64,
    pub dict: String,
    pub idx: i32,
    pub create_time: i64,
    pub is_finished: bool,
    pub words: Value,
}

#[derive(Debug, Clone)]
pub struct NewReviewRecord {
    pub user_id: String,
    pub dict: String,
    pub create_time: i64,
    pub words: Value,
}

#[derive(Debug, Clone, FromRow)]
pub struct RankedReviewWord {
    pub name: String,
    pub trans: Vec<String>,
    pub usphone: String,
    pub ukphone: String,
    pub notation: Option<String>,
}
