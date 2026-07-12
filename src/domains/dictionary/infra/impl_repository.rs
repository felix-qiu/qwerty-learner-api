use async_trait::async_trait;
use sqlx::{PgPool, Postgres, Transaction};

use crate::domains::dictionary::{
    domain::{
        model::{Dictionary, NewDictionary, NewWord, Word},
        repository::DictionaryRepository,
    },
    dto::dictionary_dto::DictionaryListQuery,
};

pub struct DictionaryRepo;

const DICTIONARY_COLUMNS: &str = r#"
    id,
    name,
    description,
    category,
    tags,
    length,
    language,
    language_category,
    default_pron_index,
    sort_order,
    is_published,
    created_at,
    updated_at,
    chapter_count
"#;

#[async_trait]
impl DictionaryRepository for DictionaryRepo {
    async fn list(
        &self,
        pool: &PgPool,
        query: &DictionaryListQuery,
    ) -> Result<(Vec<Dictionary>, i64), sqlx::Error> {
        let language_category = query.language_category;
        let category = normalized_filter(query.category.as_deref());
        let tag = normalized_filter(query.tag.as_deref());
        let keyword = normalized_filter(query.q.as_deref());

        let dictionaries = sqlx::query_as::<_, Dictionary>(&format!(
            r#"
            SELECT {DICTIONARY_COLUMNS}
              FROM dictionaries
             WHERE is_published = true
               AND ($1::text IS NULL OR language_category = $1)
               AND ($2::text IS NULL OR category = $2)
               AND ($3::text IS NULL OR $3 = ANY(tags))
               AND (
                    $4::text IS NULL
                    OR name ILIKE ('%' || $4 || '%')
                    OR description ILIKE ('%' || $4 || '%')
               )
             ORDER BY sort_order, id
            "#
        ))
        .bind(language_category)
        .bind(category)
        .bind(tag)
        .bind(keyword)
        .fetch_all(pool)
        .await?;

        let total = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
              FROM dictionaries
             WHERE is_published = true
               AND ($1::text IS NULL OR language_category = $1)
               AND ($2::text IS NULL OR category = $2)
               AND ($3::text IS NULL OR $3 = ANY(tags))
               AND (
                    $4::text IS NULL
                    OR name ILIKE ('%' || $4 || '%')
                    OR description ILIKE ('%' || $4 || '%')
               )
            "#,
        )
        .bind(language_category)
        .bind(category)
        .bind(tag)
        .bind(keyword)
        .fetch_one(pool)
        .await?;

        Ok((dictionaries, total))
    }

    async fn find_published_by_id(
        &self,
        pool: &PgPool,
        id: &str,
    ) -> Result<Option<Dictionary>, sqlx::Error> {
        sqlx::query_as::<_, Dictionary>(&format!(
            "SELECT {DICTIONARY_COLUMNS} FROM dictionaries WHERE id = $1 AND is_published = true"
        ))
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    async fn find_by_id(&self, pool: &PgPool, id: &str) -> Result<Option<Dictionary>, sqlx::Error> {
        sqlx::query_as::<_, Dictionary>(&format!(
            "SELECT {DICTIONARY_COLUMNS} FROM dictionaries WHERE id = $1"
        ))
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    async fn is_active_admin(&self, pool: &PgPool, user_id: &str) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1
                  FROM users
                 WHERE id = $1
                   AND role = 'admin'
                   AND status = 'active'
            )
            "#,
        )
        .bind(user_id)
        .fetch_one(pool)
        .await
    }

    async fn create(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        dictionary: NewDictionary,
    ) -> Result<Dictionary, sqlx::Error> {
        sqlx::query_as::<_, Dictionary>(&format!(
            r#"
            INSERT INTO dictionaries
                (id, name, description, category, tags, language, language_category, default_pron_index)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING {DICTIONARY_COLUMNS}
            "#
        ))
        .bind(dictionary.id)
        .bind(dictionary.name)
        .bind(dictionary.description)
        .bind(dictionary.category)
        .bind(dictionary.tags)
        .bind(dictionary.language)
        .bind(dictionary.language_category)
        .bind(dictionary.default_pron_index)
        .fetch_one(&mut **tx)
        .await
    }

    async fn update(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        id: &str,
        dictionary: NewDictionary,
    ) -> Result<Option<Dictionary>, sqlx::Error> {
        sqlx::query_as::<_, Dictionary>(&format!(
            r#"
            UPDATE dictionaries
               SET name = $2,
                   description = $3,
                   category = $4,
                   tags = $5,
                   language = $6,
                   language_category = $7,
                   default_pron_index = $8
             WHERE id = $1
            RETURNING {DICTIONARY_COLUMNS}
            "#
        ))
        .bind(id)
        .bind(dictionary.name)
        .bind(dictionary.description)
        .bind(dictionary.category)
        .bind(dictionary.tags)
        .bind(dictionary.language)
        .bind(dictionary.language_category)
        .bind(dictionary.default_pron_index)
        .fetch_optional(&mut **tx)
        .await
    }

    async fn delete(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        id: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM dictionaries WHERE id = $1")
            .bind(id)
            .execute(&mut **tx)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn list_words(
        &self,
        pool: &PgPool,
        dict_id: &str,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<Word>, sqlx::Error> {
        sqlx::query_as::<_, Word>(
            r#"
            SELECT idx AS index, name, trans, usphone, ukphone, notation
              FROM words
             WHERE dict_id = $1
             ORDER BY idx
             OFFSET $2
             LIMIT $3
            "#,
        )
        .bind(dict_id)
        .bind(offset)
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    async fn find_word(
        &self,
        pool: &PgPool,
        dict_id: &str,
        word_name: &str,
    ) -> Result<Option<Word>, sqlx::Error> {
        sqlx::query_as::<_, Word>(
            r#"
            SELECT idx AS index, name, trans, usphone, ukphone, notation
              FROM words
             WHERE dict_id = $1 AND name = $2
             ORDER BY idx
             LIMIT 1
            "#,
        )
        .bind(dict_id)
        .bind(word_name)
        .fetch_optional(pool)
        .await
    }

    async fn replace_words(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        dict_id: &str,
        words: Vec<NewWord>,
    ) -> Result<(i64, Dictionary), sqlx::Error> {
        sqlx::query("DELETE FROM words WHERE dict_id = $1")
            .bind(dict_id)
            .execute(&mut **tx)
            .await?;

        let written = insert_words(tx, dict_id, 0, words).await?;
        let dictionary = sync_dictionary(tx, dict_id).await?;
        Ok((written, dictionary))
    }

    async fn append_words(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        dict_id: &str,
        words: Vec<NewWord>,
    ) -> Result<(i64, Dictionary), sqlx::Error> {
        let start_index = sqlx::query_scalar::<_, i32>(
            "SELECT COALESCE(MAX(idx), -1) + 1 FROM words WHERE dict_id = $1",
        )
        .bind(dict_id)
        .fetch_one(&mut **tx)
        .await?;

        let written = insert_words(tx, dict_id, start_index, words).await?;
        let dictionary = sync_dictionary(tx, dict_id).await?;
        Ok((written, dictionary))
    }
}

fn normalized_filter(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

async fn insert_words(
    tx: &mut Transaction<'_, Postgres>,
    dict_id: &str,
    start_index: i32,
    words: Vec<NewWord>,
) -> Result<i64, sqlx::Error> {
    let written = words.len() as i64;
    for (offset, word) in words.into_iter().enumerate() {
        sqlx::query(
            r#"
            INSERT INTO words (dict_id, idx, name, trans, usphone, ukphone, notation)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(dict_id)
        .bind(start_index + offset as i32)
        .bind(word.name)
        .bind(word.trans)
        .bind(word.usphone)
        .bind(word.ukphone)
        .bind(word.notation)
        .execute(&mut **tx)
        .await?;
    }
    Ok(written)
}

async fn sync_dictionary(
    tx: &mut Transaction<'_, Postgres>,
    dict_id: &str,
) -> Result<Dictionary, sqlx::Error> {
    sqlx::query_scalar::<_, i32>("SELECT sync_dictionary_length($1)")
        .bind(dict_id)
        .fetch_one(&mut **tx)
        .await?;

    sqlx::query_as::<_, Dictionary>(&format!(
        "SELECT {DICTIONARY_COLUMNS} FROM dictionaries WHERE id = $1"
    ))
    .bind(dict_id)
    .fetch_one(&mut **tx)
    .await
}
