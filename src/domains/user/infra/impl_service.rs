use crate::{
    common::{error::AppError, hash_util},
    domains::user::{
        domain::{
            model::{NewUser, UserPatch},
            repository::UserRepository,
            service::UserServiceTrait,
        },
        dto::user_dto::{CreateUserDto, SearchUserDto, UpdateUserDto, UserDto},
        infra::impl_repository::UserRepo,
    },
};
use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct UserService {
    pub pool: PgPool,
    pub repo: Arc<dyn UserRepository + Send + Sync>,
}

#[async_trait]
impl UserServiceTrait for UserService {
    fn create_service(pool: PgPool) -> Arc<dyn UserServiceTrait> {
        Arc::new(Self {
            pool,
            repo: Arc::new(UserRepo {}),
        })
    }

    async fn get_user_by_id(&self, id: String) -> Result<UserDto, AppError> {
        match self.repo.find_by_id(self.pool.clone(), &id).await {
            Ok(Some(user)) => Ok(UserDto::from(user)),
            Ok(None) => Err(AppError::NotFound("User not found".into())),
            Err(err) => {
                tracing::error!("Error retrieving user: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    async fn get_user_list(
        &self,
        search_user_dto: SearchUserDto,
    ) -> Result<Vec<UserDto>, AppError> {
        self.repo
            .find_list(self.pool.clone(), search_user_dto)
            .await
            .map(|users| users.into_iter().map(UserDto::from).collect())
            .map_err(|err| {
                tracing::error!("Error fetching users: {err}");
                AppError::DatabaseError(err)
            })
    }

    async fn get_users(&self) -> Result<Vec<UserDto>, AppError> {
        self.repo
            .find_all(self.pool.clone())
            .await
            .map(|users| users.into_iter().map(UserDto::from).collect())
            .map_err(|err| {
                tracing::error!("Error fetching users: {err}");
                AppError::DatabaseError(err)
            })
    }

    async fn create_user(&self, create_user: CreateUserDto) -> Result<UserDto, AppError> {
        let email = normalize_email(&create_user.email)?;
        if self
            .repo
            .find_by_email(self.pool.clone(), &email)
            .await
            .map_err(AppError::DatabaseError)?
            .is_some()
        {
            return Err(AppError::Conflict("Email already registered".into()));
        }

        let role = normalize_role(create_user.role.as_deref())?;
        let status = normalize_status(create_user.status.as_deref())?;
        let password_hash =
            hash_util::hash_password(&create_user.password).map_err(|_| AppError::InternalError)?;

        let mut tx = self.pool.begin().await?;
        let user = self
            .repo
            .create(
                &mut tx,
                NewUser {
                    id: format!("usr_{}", Uuid::new_v4()),
                    email,
                    password_hash,
                    display_name: create_user.display_name,
                    avatar_url: create_user.avatar_url,
                    role,
                    status,
                },
            )
            .await
            .map_err(AppError::DatabaseError)?;

        tx.commit().await?;
        Ok(UserDto::from(user))
    }

    async fn update_user(&self, id: String, payload: UpdateUserDto) -> Result<UserDto, AppError> {
        let email = match payload.email {
            Some(email) => {
                let normalized_email = normalize_email(&email)?;
                if let Some(existing) = self
                    .repo
                    .find_by_email(self.pool.clone(), &normalized_email)
                    .await
                    .map_err(AppError::DatabaseError)?
                {
                    if existing.id != id {
                        return Err(AppError::Conflict("Email already registered".into()));
                    }
                }
                Some(normalized_email)
            }
            None => None,
        };

        let password_hash = match payload.password {
            Some(password) => {
                Some(hash_util::hash_password(&password).map_err(|_| AppError::InternalError)?)
            }
            None => None,
        };

        let role = match payload.role {
            Some(role) => Some(normalize_role(Some(&role))?),
            None => None,
        };
        let status = match payload.status {
            Some(status) => Some(normalize_status(Some(&status))?),
            None => None,
        };

        let mut tx = self.pool.begin().await?;
        let patch = UserPatch {
            email,
            password_hash,
            display_name: payload.display_name,
            avatar_url: payload.avatar_url,
            role,
            status,
        };

        match self.repo.update(&mut tx, &id, patch).await {
            Ok(Some(user)) => {
                tx.commit().await?;
                Ok(UserDto::from(user))
            }
            Ok(None) => {
                tx.rollback().await?;
                Err(AppError::NotFound("User not found".into()))
            }
            Err(err) => {
                tracing::error!("Error updating user: {err}");
                tx.rollback().await?;
                Err(AppError::DatabaseError(err))
            }
        }
    }

    async fn delete_user(&self, id: String) -> Result<String, AppError> {
        let mut tx = self.pool.begin().await?;

        match self.repo.delete(&mut tx, &id).await {
            Ok(true) => {
                tx.commit().await?;
                Ok("User deleted".into())
            }
            Ok(false) => {
                tx.rollback().await?;
                Err(AppError::NotFound("User not found".into()))
            }
            Err(err) => {
                tracing::error!("Error deleting user: {err}");
                tx.rollback().await?;
                Err(AppError::DatabaseError(err))
            }
        }
    }
}

fn normalize_email(email: &str) -> Result<String, AppError> {
    let email = email.trim().to_ascii_lowercase();
    if email.is_empty() {
        return Err(AppError::ValidationError("Email is required".into()));
    }
    Ok(email)
}

fn normalize_role(role: Option<&str>) -> Result<String, AppError> {
    match role.map(str::trim).filter(|value| !value.is_empty()) {
        Some("user") => Ok("user".to_string()),
        Some("admin") => Ok("admin".to_string()),
        Some(value) => Err(AppError::ValidationError(format!("Invalid role: {value}"))),
        None => Ok("user".to_string()),
    }
}

fn normalize_status(status: Option<&str>) -> Result<String, AppError> {
    match status.map(str::trim).filter(|value| !value.is_empty()) {
        Some("active") => Ok("active".to_string()),
        Some("disabled") => Ok("disabled".to_string()),
        Some("deleted") => Ok("deleted".to_string()),
        Some(value) => Err(AppError::ValidationError(format!(
            "Invalid status: {value}"
        ))),
        None => Ok("active".to_string()),
    }
}
