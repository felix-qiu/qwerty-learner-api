use crate::domains::user::{
    domain::{
        model::{NewUser, User, UserPatch},
        repository::UserRepository,
    },
    dto::user_dto::SearchUserDto,
};
use async_trait::async_trait;

use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};

pub struct UserRepo;

const USER_COLUMNS: &str = r#"
    id,
    email,
    password_hash,
    display_name,
    avatar_url,
    role,
    status,
    last_login_at,
    created_at,
    updated_at
"#;

const FIND_USER_QUERY: &str = r#"
    SELECT
        id,
        email,
        password_hash,
        display_name,
        avatar_url,
        role,
        status,
        last_login_at,
        created_at,
        updated_at
    FROM users
    WHERE 1=1
    "#;

#[async_trait]
impl UserRepository for UserRepo {
    async fn find_all(&self, pool: PgPool) -> Result<Vec<User>, sqlx::Error> {
        let users = sqlx::query_as::<_, User>(FIND_USER_QUERY)
            .fetch_all(&pool)
            .await?;
        Ok(users)
    }

    async fn find_by_id(&self, pool: PgPool, id: &str) -> Result<Option<User>, sqlx::Error> {
        let user = sqlx::query_as::<_, User>(&format!(
            r#"
            SELECT {USER_COLUMNS}
              FROM users
             WHERE id = $1
            "#
        ))
        .bind(id)
        .fetch_optional(&pool)
        .await?;
        Ok(user)
    }

    async fn find_by_email(&self, pool: PgPool, email: &str) -> Result<Option<User>, sqlx::Error> {
        let user = sqlx::query_as::<_, User>(&format!(
            r#"
            SELECT {USER_COLUMNS}
              FROM users
             WHERE email_normalized = lower($1)
            "#
        ))
        .bind(email)
        .fetch_optional(&pool)
        .await?;
        Ok(user)
    }

    async fn find_list(
        &self,
        pool: PgPool,
        search_user_dto: SearchUserDto,
    ) -> Result<Vec<User>, sqlx::Error> {
        let mut builder = QueryBuilder::<Postgres>::new(FIND_USER_QUERY);

        if let Some(s) = search_user_dto
            .id
            .as_deref()
            .filter(|s| !s.trim().is_empty())
        {
            builder.push(" AND id = ");
            builder.push_bind(s);
        }

        if let Some(s) = search_user_dto
            .email
            .as_deref()
            .filter(|s| !s.trim().is_empty())
        {
            builder.push(" AND email_normalized = lower(");
            builder.push_bind(s);
            builder.push(")");
        }

        if let Some(s) = search_user_dto
            .display_name
            .as_deref()
            .filter(|s| !s.trim().is_empty())
        {
            builder.push(" AND display_name ILIKE ");
            builder.push_bind(format!("%{}%", s));
        }

        if let Some(s) = search_user_dto
            .role
            .as_deref()
            .filter(|s| !s.trim().is_empty())
        {
            builder.push(" AND role = ");
            builder.push_bind(s);
        }

        if let Some(s) = search_user_dto
            .status
            .as_deref()
            .filter(|s| !s.trim().is_empty())
        {
            builder.push(" AND status = ");
            builder.push_bind(s);
        }

        let query = builder.build_query_as::<User>();
        let users = query.fetch_all(&pool).await?;
        Ok(users)
    }

    async fn create(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user: NewUser,
    ) -> Result<User, sqlx::Error> {
        let created_user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users
                (id, email, password_hash, display_name, avatar_url, role, status)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7)
            RETURNING
                id,
                email,
                password_hash,
                display_name,
                avatar_url,
                role,
                status,
                last_login_at,
                created_at,
                updated_at
            "#,
        )
        .bind(user.id)
        .bind(user.email)
        .bind(user.password_hash)
        .bind(user.display_name)
        .bind(user.avatar_url)
        .bind(user.role)
        .bind(user.status)
        .fetch_one(&mut **tx)
        .await?;

        Ok(created_user)
    }

    async fn update(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        id: &str,
        user: UserPatch,
    ) -> Result<Option<User>, sqlx::Error> {
        let existing = self.find_by_id_in_tx(tx, id).await?;

        if existing.is_some() {
            let mut builder = QueryBuilder::<Postgres>::new("UPDATE users SET updated_at = now()");

            if let Some(value) = user.email {
                builder.push(", email = ").push_bind(value);
            }
            if let Some(value) = user.password_hash {
                builder.push(", password_hash = ").push_bind(value);
            }
            if let Some(value) = user.display_name {
                builder.push(", display_name = ").push_bind(value);
            }
            if let Some(value) = user.avatar_url {
                builder.push(", avatar_url = ").push_bind(value);
            }
            if let Some(value) = user.role {
                builder.push(", role = ").push_bind(value);
            }
            if let Some(value) = user.status {
                builder.push(", status = ").push_bind(value);
            }

            builder.push(" WHERE id = ").push_bind(id);
            let query = builder.build();
            query.execute(&mut **tx).await?;

            return self.find_by_id_in_tx(tx, id).await;
        }

        Ok(None)
    }

    async fn delete(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        id: &str,
    ) -> Result<bool, sqlx::Error> {
        let res = sqlx::query(r#"DELETE FROM users WHERE id = $1"#)
            .bind(id)
            .execute(&mut **tx)
            .await?;
        Ok(res.rows_affected() > 0)
    }
}

impl UserRepo {
    async fn find_by_id_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        id: &str,
    ) -> Result<Option<User>, sqlx::Error> {
        let user = sqlx::query_as::<_, User>(&format!(
            r#"
            SELECT {USER_COLUMNS}
              FROM users
             WHERE id = $1
            "#
        ))
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?;

        Ok(user)
    }
}
