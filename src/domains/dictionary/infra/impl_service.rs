use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;

use crate::{
    common::error::AppError,
    domains::dictionary::{
        domain::{
            model::{NewDictionary, NewWord},
            repository::DictionaryRepository,
            service::DictionaryServiceTrait,
        },
        dto::dictionary_dto::{
            DictionaryCreateDto, DictionaryDto, DictionaryListQuery, DictionaryListResponse,
            WordBulkMode, WordBulkRequest, WordBulkResponse, WordListQuery, WordListResponse,
            WordWithIndexDto,
        },
        infra::impl_repository::DictionaryRepo,
    },
};

#[derive(Clone)]
pub struct DictionaryService {
    pool: PgPool,
    repo: Arc<dyn DictionaryRepository>,
}

#[async_trait]
impl DictionaryServiceTrait for DictionaryService {
    fn create_service(pool: PgPool) -> Arc<dyn DictionaryServiceTrait> {
        Arc::new(Self {
            pool,
            repo: Arc::new(DictionaryRepo),
        })
    }

    async fn list(&self, query: DictionaryListQuery) -> Result<DictionaryListResponse, AppError> {
        let (items, total) = self.repo.list(&self.pool, &query).await.map_err(db_error)?;
        Ok(DictionaryListResponse {
            items: items.into_iter().map(DictionaryDto::from).collect(),
            total,
        })
    }

    async fn get(&self, id: String) -> Result<DictionaryDto, AppError> {
        let id = required("dictId", id)?;
        self.repo
            .find_published_by_id(&self.pool, &id)
            .await
            .map_err(db_error)?
            .map(DictionaryDto::from)
            .ok_or_else(dictionary_not_found)
    }

    async fn get_words(
        &self,
        id: String,
        query: WordListQuery,
    ) -> Result<WordListResponse, AppError> {
        let id = required("dictId", id)?;
        let dictionary = self
            .repo
            .find_published_by_id(&self.pool, &id)
            .await
            .map_err(db_error)?
            .ok_or_else(dictionary_not_found)?;

        let (chapter, offset, limit) = match query.chapter {
            Some(chapter) if chapter >= 0 => (Some(chapter), chapter as i64 * 20, 20),
            Some(_) => {
                return Err(AppError::ValidationError(
                    "chapter must be greater than or equal to 0".into(),
                ))
            }
            None => {
                let offset = query.offset.unwrap_or(0);
                let limit = query.limit.unwrap_or(20);
                if offset < 0 {
                    return Err(AppError::ValidationError(
                        "offset must be greater than or equal to 0".into(),
                    ));
                }
                if !(1..=500).contains(&limit) {
                    return Err(AppError::ValidationError(
                        "limit must be between 1 and 500".into(),
                    ));
                }
                (None, offset, limit)
            }
        };

        let words = self
            .repo
            .list_words(&self.pool, &id, offset, limit)
            .await
            .map_err(db_error)?;

        Ok(WordListResponse {
            dict_id: id,
            chapter,
            chapter_count: dictionary.chapter_count,
            total: dictionary.length,
            words: words.into_iter().map(WordWithIndexDto::from).collect(),
        })
    }

    async fn get_word(&self, id: String, word_name: String) -> Result<WordWithIndexDto, AppError> {
        let id = required("dictId", id)?;
        let word_name = required("wordName", word_name)?;
        if self
            .repo
            .find_published_by_id(&self.pool, &id)
            .await
            .map_err(db_error)?
            .is_none()
        {
            return Err(dictionary_not_found());
        }

        self.repo
            .find_word(&self.pool, &id, &word_name)
            .await
            .map_err(db_error)?
            .map(WordWithIndexDto::from)
            .ok_or_else(|| AppError::NotFound("Word not found".into()))
    }

    async fn create(
        &self,
        user_id: String,
        payload: DictionaryCreateDto,
    ) -> Result<DictionaryDto, AppError> {
        self.ensure_admin(&user_id).await?;
        let dictionary = normalize_dictionary(payload)?;
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let created = self
            .repo
            .create(&mut tx, dictionary)
            .await
            .map_err(map_write_error)?;
        tx.commit().await.map_err(db_error)?;
        Ok(DictionaryDto::from(created))
    }

    async fn update(
        &self,
        user_id: String,
        id: String,
        payload: DictionaryCreateDto,
    ) -> Result<DictionaryDto, AppError> {
        self.ensure_admin(&user_id).await?;
        let id = required("dictId", id)?;
        let dictionary = normalize_dictionary(payload)?;
        if dictionary.id != id {
            return Err(AppError::ValidationError(
                "Body id must match path dictId".into(),
            ));
        }

        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let updated = self
            .repo
            .update(&mut tx, &id, dictionary)
            .await
            .map_err(map_write_error)?;
        match updated {
            Some(dictionary) => {
                tx.commit().await.map_err(db_error)?;
                Ok(DictionaryDto::from(dictionary))
            }
            None => {
                tx.rollback().await.map_err(db_error)?;
                Err(dictionary_not_found())
            }
        }
    }

    async fn bulk_words(
        &self,
        user_id: String,
        id: String,
        payload: WordBulkRequest,
    ) -> Result<WordBulkResponse, AppError> {
        self.ensure_admin(&user_id).await?;
        let id = required("dictId", id)?;
        if self
            .repo
            .find_by_id(&self.pool, &id)
            .await
            .map_err(db_error)?
            .is_none()
        {
            return Err(dictionary_not_found());
        }

        let words = payload
            .words
            .into_iter()
            .map(|word| {
                Ok(NewWord {
                    name: required("words[].name", word.name)?,
                    trans: word.trans,
                    usphone: word.usphone,
                    ukphone: word.ukphone,
                    notation: word.notation,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;

        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let (written, dictionary) = match payload.mode {
            WordBulkMode::Replace => self.repo.replace_words(&mut tx, &id, words).await,
            WordBulkMode::Append => self.repo.append_words(&mut tx, &id, words).await,
        }
        .map_err(map_write_error)?;
        tx.commit().await.map_err(db_error)?;

        Ok(WordBulkResponse {
            written,
            length: dictionary.length,
            chapter_count: dictionary.chapter_count,
        })
    }

    async fn delete(&self, user_id: String, id: String) -> Result<(), AppError> {
        self.ensure_admin(&user_id).await?;
        let id = required("dictId", id)?;
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        if self.repo.delete(&mut tx, &id).await.map_err(db_error)? {
            tx.commit().await.map_err(db_error)?;
            Ok(())
        } else {
            tx.rollback().await.map_err(db_error)?;
            Err(dictionary_not_found())
        }
    }
}

impl DictionaryService {
    async fn ensure_admin(&self, user_id: &str) -> Result<(), AppError> {
        let is_admin = self
            .repo
            .is_active_admin(&self.pool, user_id)
            .await
            .map_err(db_error)?;
        if is_admin {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }
}

fn normalize_dictionary(payload: DictionaryCreateDto) -> Result<NewDictionary, AppError> {
    Ok(NewDictionary {
        id: required("id", payload.id)?,
        name: required("name", payload.name)?,
        description: payload.description,
        category: required("category", payload.category)?,
        tags: payload.tags,
        language: payload.language,
        language_category: payload.language_category,
        default_pron_index: payload.default_pron_index,
    })
}

fn required(field: &str, value: String) -> Result<String, AppError> {
    let value = value.trim().to_string();
    if value.is_empty() {
        Err(AppError::ValidationError(format!("{field} is required")))
    } else {
        Ok(value)
    }
}

fn dictionary_not_found() -> AppError {
    AppError::NotFound("Dictionary not found".into())
}

fn db_error(error: sqlx::Error) -> AppError {
    tracing::error!("Dictionary database error: {error}");
    AppError::DatabaseError(error)
}

fn map_write_error(error: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(database_error) = &error {
        if database_error.is_unique_violation() {
            return AppError::Conflict("Dictionary already exists".into());
        }
    }
    db_error(error)
}
