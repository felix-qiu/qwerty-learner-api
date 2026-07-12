use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use async_trait::async_trait;
use sqlx::PgPool;

use crate::{
    common::error::AppError,
    domains::{
        dictionary::dto::dictionary_dto::WordDto,
        error_book::{
            domain::{
                model::{ErrorWordGroup, ErrorWordRecord},
                repository::{ErrorBookRepository, ErrorWordFilter},
                service::ErrorBookServiceTrait,
            },
            dto::error_book_dto::{
                DictErrorWordsResponse, ErrorBookListQuery, ErrorBookListResponse, ErrorWordDataDto,
            },
            infra::impl_repository::ErrorBookRepo,
        },
        record::dto::record_dto::{LetterMistakes, WordRecordDto},
    },
};

#[derive(Clone)]
pub struct ErrorBookService {
    pool: PgPool,
    repo: Arc<dyn ErrorBookRepository>,
}

#[async_trait]
impl ErrorBookServiceTrait for ErrorBookService {
    fn create_service(pool: PgPool) -> Arc<dyn ErrorBookServiceTrait> {
        Arc::new(Self {
            pool,
            repo: Arc::new(ErrorBookRepo),
        })
    }

    async fn list_error_book(
        &self,
        user_id: String,
        query: ErrorBookListQuery,
    ) -> Result<ErrorBookListResponse, AppError> {
        let (offset, page_size) = pagination(query.page, query.page_size)?;
        let ascending = parse_sort(query.sort.as_deref())?;
        let include_records = query.include_records.unwrap_or(false);
        let filter = ErrorWordFilter {
            dict: normalize_optional(query.dict),
            offset,
            limit: page_size,
            ascending,
        };

        self.ensure_active_user(&user_id).await?;
        let (groups, total) = self
            .repo
            .list_groups(&self.pool, &user_id, &filter)
            .await
            .map_err(db_error)?;
        let items = self.build_items(&user_id, groups, include_records).await?;
        Ok(ErrorBookListResponse { items, total })
    }

    async fn get_dictionary_error_words(
        &self,
        user_id: String,
        dict_id: String,
    ) -> Result<DictErrorWordsResponse, AppError> {
        let dict_id = required("dictId", dict_id)?;
        self.ensure_active_user(&user_id).await?;
        if !self
            .repo
            .is_published_dictionary(&self.pool, &dict_id)
            .await
            .map_err(db_error)?
        {
            return Err(AppError::NotFound("Dictionary not found".into()));
        }
        let filter = ErrorWordFilter {
            dict: Some(dict_id),
            offset: 0,
            limit: i64::MAX,
            ascending: true,
        };
        let (groups, _) = self
            .repo
            .list_groups(&self.pool, &user_id, &filter)
            .await
            .map_err(db_error)?;
        let items = self.build_items(&user_id, groups, false).await?;
        Ok(DictErrorWordsResponse { items })
    }
}

impl ErrorBookService {
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

    async fn build_items(
        &self,
        user_id: &str,
        groups: Vec<ErrorWordGroup>,
        include_records: bool,
    ) -> Result<Vec<ErrorWordDataDto>, AppError> {
        let keys = groups
            .iter()
            .map(|group| (group.dict.clone(), group.word.clone()))
            .collect::<Vec<_>>();
        let records = self
            .repo
            .list_records(&self.pool, user_id, &keys)
            .await
            .map_err(db_error)?;
        let mut records_by_group = HashMap::<(String, String), Vec<ErrorWordRecord>>::new();
        for record in records {
            records_by_group
                .entry((record.dict.clone(), record.word.clone()))
                .or_default()
                .push(record);
        }

        groups
            .into_iter()
            .map(|group| {
                let records = records_by_group
                    .remove(&(group.dict.clone(), group.word.clone()))
                    .unwrap_or_default();
                error_word_dto(group, records, include_records)
            })
            .collect()
    }
}

fn error_word_dto(
    group: ErrorWordGroup,
    records: Vec<ErrorWordRecord>,
    include_records: bool,
) -> Result<ErrorWordDataDto, AppError> {
    let mut error_letters = BTreeMap::<String, i32>::new();
    for record in &records {
        let mistakes = serde_json::from_value::<LetterMistakes>(record.mistakes.clone())
            .map_err(|_| AppError::InternalError)?;
        for (index, mistakes) in mistakes.0 {
            if !mistakes.is_empty() {
                let count = i32::try_from(mistakes.len()).map_err(|_| AppError::InternalError)?;
                let total = error_letters.entry(index).or_default();
                *total = total.checked_add(count).ok_or(AppError::InternalError)?;
            }
        }
    }
    let error_char = error_characters(&group.word, &error_letters);
    let record_dtos = records
        .into_iter()
        .map(word_record_dto)
        .collect::<Result<Vec<_>, _>>()?;
    let records = include_records.then_some(record_dtos);
    let wrong_count = group.wrong_count;

    Ok(ErrorWordDataDto {
        word: group.word.clone(),
        dict: group.dict,
        wrong_count,
        error_count: wrong_count,
        latest_error_time: group.latest_error_time,
        error_letters,
        error_char,
        origin_data: WordDto {
            name: group.word,
            trans: group.trans,
            usphone: group.usphone,
            ukphone: group.ukphone,
            notation: group.notation,
        },
        records,
    })
}

fn error_characters(word: &str, error_letters: &BTreeMap<String, i32>) -> Vec<String> {
    let characters = word.chars().collect::<Vec<_>>();
    let mut ranked = error_letters
        .iter()
        .filter_map(|(index, count)| {
            let index = index.parse::<usize>().ok()?;
            let character = characters.get(index)?;
            Some((*count, index, character.to_string()))
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    ranked
        .into_iter()
        .map(|(_, _, character)| character)
        .collect()
}

fn word_record_dto(record: ErrorWordRecord) -> Result<WordRecordDto, AppError> {
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

fn parse_sort(sort: Option<&str>) -> Result<bool, AppError> {
    match sort.unwrap_or("wrongCount:asc") {
        "wrongCount:asc" => Ok(true),
        "wrongCount:desc" => Ok(false),
        _ => Err(AppError::ValidationError(
            "sort must be wrongCount:asc or wrongCount:desc".into(),
        )),
    }
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
    tracing::error!("Error Book database error: {error}");
    AppError::DatabaseError(error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn error_characters_are_sorted_by_count_then_index() {
        let errors = BTreeMap::from([
            ("2".to_string(), 3),
            ("0".to_string(), 3),
            ("1".to_string(), 5),
        ]);
        assert_eq!(error_characters("cat", &errors), vec!["a", "c", "t"]);
    }

    #[test]
    fn pagination_and_sort_are_strict() {
        assert_eq!(pagination(None, None).unwrap(), (0, 20));
        assert!(pagination(Some(i64::MAX), Some(100)).is_err());
        assert!(parse_sort(Some("timeStamp:desc")).is_err());
    }

    #[test]
    fn error_word_dto_aggregates_mistakes_and_controls_records() {
        let group = ErrorWordGroup {
            word: "cat".into(),
            dict: "test".into(),
            wrong_count: 3,
            latest_error_time: 20,
            trans: vec!["猫".into()],
            usphone: String::new(),
            ukphone: String::new(),
            notation: None,
        };
        let records = vec![
            ErrorWordRecord {
                id: 1,
                word: "cat".into(),
                dict: "test".into(),
                chapter: Some(0),
                timing: vec![10.0, 20.0],
                wrong_count: 1,
                mistakes: json!({"0": ["x"], "2": ["z"]}),
                time_stamp: 10,
            },
            ErrorWordRecord {
                id: 2,
                word: "cat".into(),
                dict: "test".into(),
                chapter: Some(0),
                timing: vec![30.0],
                wrong_count: 2,
                mistakes: json!({"0": ["q", "w"]}),
                time_stamp: 20,
            },
        ];

        let without_records = error_word_dto(group.clone(), records.clone(), false).unwrap();
        assert_eq!(without_records.wrong_count, 3);
        assert_eq!(without_records.error_count, 3);
        assert_eq!(without_records.error_letters.get("0"), Some(&3));
        assert_eq!(without_records.error_char, vec!["c", "t"]);
        assert!(without_records.records.is_none());

        let with_records = error_word_dto(group, records, true).unwrap();
        assert_eq!(with_records.records.unwrap()[0].total_time, 30.0);
    }
}
