use crate::domains::user::{
    domain::model::{NewUser, User, UserPatch},
    dto::user_dto::SearchUserDto,
};

use async_trait::async_trait;
use sqlx::{PgPool, Postgres, Transaction};

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_all(&self, pool: PgPool) -> Result<Vec<User>, sqlx::Error>;

    async fn find_by_id(&self, pool: PgPool, id: &str) -> Result<Option<User>, sqlx::Error>;

    async fn find_by_email(&self, pool: PgPool, email: &str) -> Result<Option<User>, sqlx::Error>;

    async fn find_list(
        &self,
        pool: PgPool,
        search_user_dto: SearchUserDto,
    ) -> Result<Vec<User>, sqlx::Error>;

    async fn create(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user: NewUser,
    ) -> Result<User, sqlx::Error>;

    async fn update(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        id: &str,
        user: UserPatch,
    ) -> Result<Option<User>, sqlx::Error>;

    async fn delete(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        id: &str,
    ) -> Result<bool, sqlx::Error>;
}
