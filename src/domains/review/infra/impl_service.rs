use std::{cmp::Ordering, sync::Arc};

use async_trait::async_trait;
use chrono::Utc;
use sqlx::PgPool;

use crate::{
    common::error::AppError,
    domains::{
        dictionary::dto::dictionary_dto::WordDto,
        review::{
            domain::{
                model::{NewReviewRecord, RankedReviewWord, ReviewRecord},
                repository::{ReviewListFilter, ReviewPatch, ReviewRepository},
                service::ReviewServiceTrait,
            },
            dto::review_dto::{
                LatestReviewQuery, LatestReviewResponse, ReviewCreateDto, ReviewErrorDataDto,
                ReviewListQuery, ReviewListResponse, ReviewRecordDto, ReviewUpdateDto,
            },
            infra::impl_repository::ReviewRepo,
        },
    },
};

#[derive(Clone)]
pub struct ReviewService {
    pool: PgPool,
    repo: Arc<dyn ReviewRepository>,
}

#[async_trait]
impl ReviewServiceTrait for ReviewService {
    fn create_service(pool: PgPool) -> Arc<dyn ReviewServiceTrait> {
        Arc::new(Self {
            pool,
            repo: Arc::new(ReviewRepo),
        })
    }

    async fn list_reviews(
        &self,
        user_id: String,
        query: ReviewListQuery,
    ) -> Result<ReviewListResponse, AppError> {
        let (offset, page_size) = pagination(query.page, query.page_size)?;
        let filter = ReviewListFilter {
            dict: normalize_optional(query.dict),
            offset,
            limit: page_size,
        };
        self.ensure_active_user(&user_id).await?;
        let (records, total) = self
            .repo
            .list(&self.pool, &user_id, &filter)
            .await
            .map_err(db_error)?;
        let items = records
            .into_iter()
            .map(review_record_dto)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ReviewListResponse { items, total })
    }

    async fn create_review(
        &self,
        user_id: String,
        payload: ReviewCreateDto,
    ) -> Result<ReviewRecordDto, AppError> {
        let dict = required("dict", payload.dict)?;
        self.ensure_active_user(&user_id).await?;

        let words = match payload.error_data {
            Some(error_data) => rank_client_words(error_data)?,
            None => self
                .repo
                .rank_words(&self.pool, &user_id, &dict)
                .await
                .map_err(db_error)?
                .into_iter()
                .map(word_dto)
                .collect(),
        };
        let words = serde_json::to_value(words).map_err(|_| AppError::InternalError)?;
        let record = NewReviewRecord {
            user_id,
            dict,
            create_time: Utc::now().timestamp(),
            words,
        };

        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let created = self.repo.create(&mut tx, record).await.map_err(db_error)?;
        tx.commit().await.map_err(db_error)?;
        review_record_dto(created)
    }

    async fn latest_review(
        &self,
        user_id: String,
        query: LatestReviewQuery,
    ) -> Result<LatestReviewResponse, AppError> {
        let dict = required("dict", query.dict)?;
        self.ensure_active_user(&user_id).await?;
        let latest = self
            .repo
            .latest(&self.pool, &user_id, &dict)
            .await
            .map_err(db_error)?;
        let record = match latest {
            Some(record) if !record.is_finished => Some(review_record_dto(record)?),
            _ => None,
        };
        Ok(LatestReviewResponse { record })
    }

    async fn update_review(
        &self,
        user_id: String,
        review_id: i64,
        payload: ReviewUpdateDto,
    ) -> Result<ReviewRecordDto, AppError> {
        if review_id < 1 {
            return Err(AppError::ValidationError(
                "reviewId must be greater than or equal to 1".into(),
            ));
        }
        if payload.index.is_some_and(|index| index < 0) {
            return Err(AppError::ValidationError(
                "index must be greater than or equal to 0".into(),
            ));
        }
        self.ensure_active_user(&user_id).await?;
        let words = payload
            .words
            .map(serde_json::to_value)
            .transpose()
            .map_err(|_| AppError::InternalError)?;
        let patch = ReviewPatch {
            index: payload.index,
            is_finished: payload.is_finished,
            words,
        };
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let updated = self
            .repo
            .update(&mut tx, &user_id, review_id, patch)
            .await
            .map_err(db_error)?
            .ok_or_else(|| AppError::NotFound("Review record not found".into()))?;
        tx.commit().await.map_err(db_error)?;
        review_record_dto(updated)
    }
}

impl ReviewService {
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

fn rank_client_words(error_data: Vec<ReviewErrorDataDto>) -> Result<Vec<WordDto>, AppError> {
    if error_data.iter().any(|item| item.error_count < 0) {
        return Err(AppError::ValidationError(
            "errorCount must be greater than or equal to 0".into(),
        ));
    }

    let error_counts = error_data
        .iter()
        .map(|item| i64::from(item.error_count))
        .collect::<Vec<_>>();
    let latest_error_times = error_data
        .iter()
        .map(|item| item.latest_error_time)
        .collect::<Vec<_>>();
    let error_count_ranks = ranks(&error_counts);
    let latest_error_time_ranks = ranks(&latest_error_times);
    let mut ranked = error_data
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            let score =
                error_count_ranks[index] as f64 * 0.6 + latest_error_time_ranks[index] as f64 * 0.4;
            (score, index, item.origin_data)
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        left.0
            .partial_cmp(&right.0)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.1.cmp(&right.1))
    });
    Ok(ranked.into_iter().map(|(_, _, word)| word).collect())
}

fn ranks(values: &[i64]) -> Vec<usize> {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    values
        .iter()
        .map(|value| sorted.partition_point(|candidate| candidate < value) + 1)
        .collect()
}

fn review_record_dto(record: ReviewRecord) -> Result<ReviewRecordDto, AppError> {
    let words = serde_json::from_value::<Vec<WordDto>>(record.words)
        .map_err(|_| AppError::InternalError)?;
    Ok(ReviewRecordDto {
        id: record.id,
        dict: record.dict,
        index: record.idx,
        create_time: record.create_time,
        is_finished: record.is_finished,
        words,
    })
}

fn word_dto(word: RankedReviewWord) -> WordDto {
    WordDto {
        name: word.name,
        trans: word.trans,
        usphone: word.usphone,
        ukphone: word.ukphone,
        notation: word.notation,
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
    let offset = (page - 1).checked_mul(page_size).ok_or_else(|| {
        AppError::ValidationError("page and pageSize produce an invalid offset".into())
    })?;
    Ok((offset, page_size))
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
    tracing::error!("Reviews database error: {error}");
    AppError::DatabaseError(error)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn error_data(name: &str, error_count: i32, latest_error_time: i64) -> ReviewErrorDataDto {
        ReviewErrorDataDto {
            word: name.into(),
            error_count,
            latest_error_time,
            origin_data: WordDto {
                name: name.into(),
                trans: vec![],
                usphone: String::new(),
                ukphone: String::new(),
                notation: None,
            },
        }
    }

    #[test]
    fn client_words_use_weighted_rank_order() {
        let words = rank_client_words(vec![
            error_data("recent-frequent", 5, 300),
            error_data("old-rare", 1, 100),
            error_data("middle", 3, 200),
        ])
        .unwrap();
        assert_eq!(
            words.into_iter().map(|word| word.name).collect::<Vec<_>>(),
            vec!["old-rare", "middle", "recent-frequent"]
        );
    }

    #[test]
    fn equal_values_receive_equal_rank() {
        assert_eq!(ranks(&[5_i64, 1, 5, 3]), vec![3, 1, 3, 2]);
    }

    #[test]
    fn null_words_deserializes_as_no_update() {
        let update = serde_json::from_str::<ReviewUpdateDto>(r#"{"words":null}"#).unwrap();
        assert!(update.words.is_none());
    }

    #[test]
    fn null_is_rejected_for_non_nullable_optional_fields() {
        assert!(
            serde_json::from_str::<ReviewCreateDto>(r#"{"dict":"cet4","errorData":null}"#).is_err()
        );
        assert!(serde_json::from_str::<ReviewUpdateDto>(r#"{"index":null}"#).is_err());
        assert!(serde_json::from_str::<ReviewUpdateDto>(r#"{"isFinished":null}"#).is_err());
    }

    #[test]
    fn pagination_rejects_offset_overflow() {
        assert!(pagination(Some(i64::MAX), Some(100)).is_err());
    }
}
