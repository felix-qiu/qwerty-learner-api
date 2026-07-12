use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use sqlx::PgPool;

use crate::{
    common::error::AppError,
    domains::record::{
        domain::{
            model::{ChapterRecord, NewChapterRecord, NewWordRecord, WordRecord},
            repository::{ChapterFilter, ChapterRecordFilter, RecordRepository, WordRecordFilter},
            service::RecordServiceTrait,
        },
        dto::record_dto::{
            ChapterRecordCreateDto, ChapterRecordDto, ChapterRecordListQuery,
            ChapterRecordListResponse, DeleteWordRecordsQuery, DeleteWordRecordsResponse,
            LetterMistakes, WordRecordBatchRequest, WordRecordBatchResponse, WordRecordCreateDto,
            WordRecordDto, WordRecordListQuery, WordRecordListResponse,
        },
        infra::impl_repository::RecordRepo,
    },
};

#[derive(Clone)]
pub struct RecordService {
    pool: PgPool,
    repo: Arc<dyn RecordRepository>,
}

#[async_trait]
impl RecordServiceTrait for RecordService {
    fn create_service(pool: PgPool) -> Arc<dyn RecordServiceTrait> {
        Arc::new(Self {
            pool,
            repo: Arc::new(RecordRepo),
        })
    }

    async fn create_word_record(
        &self,
        user_id: String,
        payload: WordRecordCreateDto,
    ) -> Result<WordRecordDto, AppError> {
        let record = normalize_word_record(user_id, payload)?;
        self.ensure_active_user(&record.user_id).await?;
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let created = self
            .repo
            .create_word_record(&mut tx, record)
            .await
            .map_err(db_error)?;
        tx.commit().await.map_err(db_error)?;
        word_record_dto(created)
    }

    async fn batch_create_word_records(
        &self,
        user_id: String,
        payload: WordRecordBatchRequest,
    ) -> Result<WordRecordBatchResponse, AppError> {
        let item_count = payload.items.len();
        let records = payload
            .items
            .into_iter()
            .filter_map(|item| normalize_word_record(user_id.clone(), item).ok())
            .collect::<Vec<_>>();
        let rejected = item_count - records.len();

        self.ensure_active_user(&user_id).await?;

        if records.is_empty() {
            return Ok(WordRecordBatchResponse {
                ids: Vec::new(),
                accepted: 0,
                rejected,
            });
        }

        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let mut ids = Vec::with_capacity(records.len());
        for record in records {
            let created = self
                .repo
                .create_word_record(&mut tx, record)
                .await
                .map_err(db_error)?;
            ids.push(created.id);
        }
        tx.commit().await.map_err(db_error)?;

        Ok(WordRecordBatchResponse {
            accepted: ids.len(),
            rejected,
            ids,
        })
    }

    async fn list_word_records(
        &self,
        user_id: String,
        query: WordRecordListQuery,
    ) -> Result<WordRecordListResponse, AppError> {
        let (page, page_size) = pagination(query.page, query.page_size)?;
        let ascending = match query.sort.as_deref().unwrap_or("timeStamp:desc") {
            "timeStamp:asc" => true,
            "timeStamp:desc" => false,
            _ => {
                return Err(AppError::ValidationError(
                    "sort must be timeStamp:asc or timeStamp:desc".into(),
                ))
            }
        };
        validate_time_range(query.from, query.to)?;
        let chapter = parse_chapter_filter(query.chapter)?;

        let filter = WordRecordFilter {
            dict: normalize_optional(query.dict),
            chapter,
            word: normalize_optional(query.word),
            wrong_only: query.wrong_only.unwrap_or(false),
            from: query.from,
            to: query.to,
            offset: (page - 1) * page_size,
            limit: page_size,
            ascending,
        };
        self.ensure_active_user(&user_id).await?;
        let (records, total) = self
            .repo
            .list_word_records(&self.pool, &user_id, &filter)
            .await
            .map_err(db_error)?;
        let items = records
            .into_iter()
            .map(word_record_dto)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(WordRecordListResponse {
            items,
            total,
            page,
            page_size,
        })
    }

    async fn delete_word_records(
        &self,
        user_id: String,
        query: DeleteWordRecordsQuery,
    ) -> Result<DeleteWordRecordsResponse, AppError> {
        let word = required("word", query.word)?;
        let dict = required("dict", query.dict)?;
        self.ensure_active_user(&user_id).await?;
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let deleted_count = self
            .repo
            .delete_word_records(&mut tx, &user_id, &word, &dict)
            .await
            .map_err(db_error)?;
        tx.commit().await.map_err(db_error)?;
        Ok(DeleteWordRecordsResponse { deleted_count })
    }

    async fn create_chapter_record(
        &self,
        user_id: String,
        payload: ChapterRecordCreateDto,
    ) -> Result<ChapterRecordDto, AppError> {
        let record = normalize_chapter_record(user_id, payload)?;
        self.ensure_active_user(&record.user_id).await?;
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let created = self
            .repo
            .create_chapter_record(&mut tx, record)
            .await
            .map_err(db_error)?;
        tx.commit().await.map_err(db_error)?;
        Ok(chapter_record_dto(created))
    }

    async fn list_chapter_records(
        &self,
        user_id: String,
        query: ChapterRecordListQuery,
    ) -> Result<ChapterRecordListResponse, AppError> {
        let (page, page_size) = pagination(query.page, query.page_size)?;
        validate_time_range(query.from, query.to)?;
        let chapter = parse_chapter_filter(query.chapter)?;
        let filter = ChapterRecordFilter {
            dict: normalize_optional(query.dict),
            chapter,
            from: query.from,
            to: query.to,
            offset: (page - 1) * page_size,
            limit: page_size,
        };
        self.ensure_active_user(&user_id).await?;
        let (records, total) = self
            .repo
            .list_chapter_records(&self.pool, &user_id, &filter)
            .await
            .map_err(db_error)?;

        Ok(ChapterRecordListResponse {
            items: records.into_iter().map(chapter_record_dto).collect(),
            total,
            page,
            page_size,
        })
    }
}

impl RecordService {
    async fn ensure_active_user(&self, user_id: &str) -> Result<(), AppError> {
        if self
            .repo
            .is_active_user(&self.pool, user_id)
            .await
            .map_err(db_error)?
        {
            Ok(())
        } else {
            Err(AppError::InvalidToken)
        }
    }
}

fn normalize_word_record(
    user_id: String,
    payload: WordRecordCreateDto,
) -> Result<NewWordRecord, AppError> {
    validate_optional_chapter(payload.chapter)?;
    if payload.wrong_count < 0 {
        return Err(AppError::ValidationError(
            "wrongCount must be greater than or equal to 0".into(),
        ));
    }
    Ok(NewWordRecord {
        user_id,
        word: required("word", payload.word)?,
        dict: required("dict", payload.dict)?,
        chapter: payload.chapter,
        timing: payload.timing,
        wrong_count: payload.wrong_count,
        mistakes: serde_json::to_value(payload.mistakes).map_err(|_| AppError::InternalError)?,
        time_stamp: payload.time_stamp.unwrap_or_else(|| Utc::now().timestamp()),
    })
}

fn normalize_chapter_record(
    user_id: String,
    payload: ChapterRecordCreateDto,
) -> Result<NewChapterRecord, AppError> {
    validate_optional_chapter(payload.chapter)?;
    if payload.time < 0 {
        return Err(AppError::ValidationError(
            "time must be greater than or equal to 0".into(),
        ));
    }
    Ok(NewChapterRecord {
        user_id,
        dict: required("dict", payload.dict)?,
        chapter: payload.chapter,
        time_stamp: payload.time_stamp.unwrap_or_else(|| Utc::now().timestamp()),
        time_seconds: payload.time,
        correct_count: payload.correct_count,
        wrong_count: payload.wrong_count,
        word_count: payload.word_count,
        correct_word_indexes: payload.correct_word_indexes,
        word_number: payload.word_number,
        word_record_ids: payload.word_record_ids,
    })
}

fn word_record_dto(record: WordRecord) -> Result<WordRecordDto, AppError> {
    let mistakes = serde_json::from_value::<LetterMistakes>(record.mistakes)
        .map_err(|_| AppError::InternalError)?;
    let total_time = record.timing.iter().sum();
    Ok(WordRecordDto {
        id: record.id,
        word: record.word,
        dict: record.dict,
        chapter: record.chapter,
        timing: record.timing,
        wrong_count: record.wrong_count,
        mistakes,
        time_stamp: record.time_stamp,
        total_time,
    })
}

fn chapter_record_dto(record: ChapterRecord) -> ChapterRecordDto {
    let wpm = if record.time_seconds == 0 {
        0
    } else {
        (record.word_count as f64 / record.time_seconds as f64 * 60.0).round() as i32
    };
    let word_accuracy = if record.word_number == 0 {
        0
    } else {
        (record.correct_word_indexes.len() as f64 / record.word_number as f64 * 100.0).round()
            as i32
    };
    ChapterRecordDto {
        id: record.id,
        wpm,
        word_accuracy,
        dict: record.dict,
        chapter: record.chapter,
        time: record.time_seconds,
        correct_count: record.correct_count,
        wrong_count: record.wrong_count,
        word_count: record.word_count,
        correct_word_indexes: record.correct_word_indexes,
        word_number: record.word_number,
        word_record_ids: record.word_record_ids,
        time_stamp: record.time_stamp,
    }
}

fn pagination(page: Option<i64>, page_size: Option<i64>) -> Result<(i64, i64), AppError> {
    let page = page.unwrap_or(1);
    let page_size = page_size.unwrap_or(20);
    if page < 1 {
        return Err(AppError::ValidationError("page must be at least 1".into()));
    }
    if !(1..=100).contains(&page_size) {
        return Err(AppError::ValidationError(
            "pageSize must be between 1 and 100".into(),
        ));
    }
    Ok((page, page_size))
}

fn validate_optional_chapter(chapter: Option<i32>) -> Result<(), AppError> {
    if chapter.is_some_and(|chapter| chapter < -1) {
        return Err(AppError::ValidationError(
            "chapter must be null, -1, or greater than or equal to 0".into(),
        ));
    }
    Ok(())
}

fn parse_chapter_filter(chapter: Option<String>) -> Result<ChapterFilter, AppError> {
    let Some(chapter) = chapter else {
        return Ok(ChapterFilter::Any);
    };
    let chapter = chapter.trim();
    if chapter == "null" {
        return Ok(ChapterFilter::Null);
    }
    let chapter = chapter.parse::<i32>().map_err(|_| {
        AppError::ValidationError("chapter must be null, -1, or an integer >= 0".into())
    })?;
    validate_optional_chapter(Some(chapter))?;
    Ok(ChapterFilter::Value(chapter))
}

fn validate_time_range(from: Option<i64>, to: Option<i64>) -> Result<(), AppError> {
    if matches!((from, to), (Some(from), Some(to)) if from > to) {
        return Err(AppError::ValidationError(
            "from must be less than or equal to to".into(),
        ));
    }
    Ok(())
}

fn required(field: &str, value: String) -> Result<String, AppError> {
    let value = value.trim().to_string();
    if value.is_empty() {
        Err(AppError::ValidationError(format!("{field} is required")))
    } else {
        Ok(value)
    }
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn db_error(error: sqlx::Error) -> AppError {
    tracing::error!("Records database error: {error}");
    AppError::DatabaseError(error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chapter_filter_supports_any_null_and_values() {
        assert_eq!(parse_chapter_filter(None).unwrap(), ChapterFilter::Any);
        assert_eq!(
            parse_chapter_filter(Some("null".into())).unwrap(),
            ChapterFilter::Null
        );
        assert_eq!(
            parse_chapter_filter(Some("-1".into())).unwrap(),
            ChapterFilter::Value(-1)
        );
        assert_eq!(
            parse_chapter_filter(Some("2".into())).unwrap(),
            ChapterFilter::Value(2)
        );
        assert!(parse_chapter_filter(Some("-2".into())).is_err());
    }
}
