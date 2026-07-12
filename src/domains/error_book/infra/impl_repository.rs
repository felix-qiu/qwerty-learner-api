use async_trait::async_trait;
use sqlx::PgPool;

use crate::domains::error_book::domain::{
    model::{ErrorWordGroup, ErrorWordRecord},
    repository::{ErrorBookRepository, ErrorWordFilter},
};

pub struct ErrorBookRepo;

#[async_trait]
impl ErrorBookRepository for ErrorBookRepo {
    async fn is_active_user(&self, pool: &PgPool, user_id: &str) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM users WHERE id = $1 AND status = 'active')",
        )
        .bind(user_id)
        .fetch_one(pool)
        .await
    }

    async fn is_published_dictionary(
        &self,
        pool: &PgPool,
        dict_id: &str,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM dictionaries WHERE id = $1 AND is_published = true)",
        )
        .bind(dict_id)
        .fetch_one(pool)
        .await
    }

    async fn list_groups(
        &self,
        pool: &PgPool,
        user_id: &str,
        filter: &ErrorWordFilter,
    ) -> Result<(Vec<ErrorWordGroup>, i64), sqlx::Error> {
        let direction = if filter.ascending { "ASC" } else { "DESC" };
        let groups = sqlx::query_as::<_, ErrorWordGroup>(&format!(
            r#"
            SELECT errors.word,
                   errors.dict,
                   errors.wrong_count,
                   errors.latest_error_time,
                   origin.trans,
                   origin.usphone,
                   origin.ukphone,
                   origin.notation
              FROM v_error_words errors
              JOIN LATERAL (
                    SELECT word.trans, word.usphone, word.ukphone, word.notation
                      FROM words word
                     WHERE word.dict_id = errors.dict AND word.name = errors.word
                     ORDER BY word.idx
                     LIMIT 1
              ) origin ON true
             WHERE errors.user_id = $1
               AND ($2::text IS NULL OR errors.dict = $2)
             ORDER BY errors.wrong_count {direction}, errors.dict ASC, errors.word ASC
             OFFSET $3
             LIMIT $4
            "#
        ))
        .bind(user_id)
        .bind(filter.dict.as_deref())
        .bind(filter.offset)
        .bind(filter.limit)
        .fetch_all(pool)
        .await?;

        let total = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
              FROM v_error_words errors
             WHERE errors.user_id = $1
               AND ($2::text IS NULL OR errors.dict = $2)
               AND EXISTS(
                    SELECT 1
                      FROM words word
                     WHERE word.dict_id = errors.dict AND word.name = errors.word
               )
            "#,
        )
        .bind(user_id)
        .bind(filter.dict.as_deref())
        .fetch_one(pool)
        .await?;

        Ok((groups, total))
    }

    async fn list_records(
        &self,
        pool: &PgPool,
        user_id: &str,
        groups: &[(String, String)],
    ) -> Result<Vec<ErrorWordRecord>, sqlx::Error> {
        if groups.is_empty() {
            return Ok(Vec::new());
        }
        let dictionaries = groups
            .iter()
            .map(|(dict, _)| dict.clone())
            .collect::<Vec<_>>();
        let words = groups
            .iter()
            .map(|(_, word)| word.clone())
            .collect::<Vec<_>>();

        sqlx::query_as::<_, ErrorWordRecord>(
            r#"
            SELECT record.id,
                   record.word,
                   record.dict,
                   record.chapter,
                   record.timing,
                   record.wrong_count,
                   record.mistakes,
                   record.time_stamp
              FROM word_records record
              JOIN UNNEST($2::text[], $3::text[]) selected(dict, word)
                ON selected.dict = record.dict AND selected.word = record.word
             WHERE record.user_id = $1 AND record.wrong_count > 0
             ORDER BY record.time_stamp DESC, record.id DESC
            "#,
        )
        .bind(user_id)
        .bind(dictionaries)
        .bind(words)
        .fetch_all(pool)
        .await
    }
}
