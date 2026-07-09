use crate::{
    common::error::AppError,
    domains::user::dto::user_dto::{CreateUserDto, SearchUserDto, UpdateUserDto, UserDto},
};

use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;

#[async_trait]
pub trait UserServiceTrait: Send + Sync {
    fn create_service(pool: PgPool) -> Arc<dyn UserServiceTrait>
    where
        Self: Sized;

    async fn get_user_by_id(&self, id: String) -> Result<UserDto, AppError>;

    async fn get_user_list(&self, search_user_dto: SearchUserDto)
        -> Result<Vec<UserDto>, AppError>;

    async fn get_users(&self) -> Result<Vec<UserDto>, AppError>;

    async fn create_user(&self, create_user: CreateUserDto) -> Result<UserDto, AppError>;

    async fn update_user(&self, id: String, payload: UpdateUserDto) -> Result<UserDto, AppError>;

    async fn delete_user(&self, id: String) -> Result<String, AppError>;
}
