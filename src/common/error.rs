use axum::{
    extract::rejection::{JsonRejection, QueryRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
    BoxError,
};

use serde::{Deserialize, Serialize};
use sqlx::Error as SqlxError;
use thiserror::Error;
use tracing::error;
use utoipa::ToSchema;

use crate::common::dto::RestApiResponse;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// AppError is an enum that represents various types of errors that can occur in the application.
/// It implements the `std::error::Error` trait and the `axum::response::IntoResponse` trait.
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] SqlxError), // Used for database-related errors

    #[error("Not found: {0}")]
    NotFound(String), // Used for not found errors

    #[error("Internal server error")]
    InternalError,

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Forbidden Request")]
    Forbidden,

    #[error("External service error: {0}")]
    ExternalServiceError(String),

    /// Used for file-related errors
    #[error("File data is empty")]
    InvalidFileData,

    #[error("File size exceeded")]
    FileSizeExceeded,

    #[error("Invalid file name")]
    InvalidFileName,

    #[error("Unsupported file extension")]
    UnsupportedFileExtension,

    /// Used for authentication-related errors
    #[error("Wrong credentials")]
    WrongCredentials,
    #[error("Missing credentials")]
    MissingCredentials,
    #[error("Invalid token")]
    InvalidToken,
    #[error("Token creation error")]
    TokenCreation,
    #[error("User not found")]
    UserNotFound,
}

/// Converts the AppError enum into an HTTP response.
/// It maps the error to an appropriate HTTP status code and constructs a JSON response body.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            AppError::ValidationError(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Forbidden => StatusCode::FORBIDDEN,
            AppError::ExternalServiceError(_) => StatusCode::BAD_GATEWAY,
            AppError::InvalidFileData
            | AppError::FileSizeExceeded
            | AppError::InvalidFileName
            | AppError::UnsupportedFileExtension => StatusCode::BAD_REQUEST,
            AppError::WrongCredentials => StatusCode::UNAUTHORIZED,
            AppError::MissingCredentials => StatusCode::BAD_REQUEST,
            AppError::InvalidToken => StatusCode::UNAUTHORIZED,
            AppError::TokenCreation => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::UserNotFound => StatusCode::NOT_FOUND,
        };

        let code = match &self {
            AppError::ValidationError(_) => "VALIDATION_ERROR",
            AppError::BadRequest(_) => "BAD_REQUEST",
            AppError::DatabaseError(_) | AppError::InternalError | AppError::TokenCreation => {
                "INTERNAL_ERROR"
            }
            AppError::NotFound(_) | AppError::UserNotFound => "NOT_FOUND",
            AppError::Conflict(_) => "CONFLICT",
            AppError::Forbidden => "FORBIDDEN",
            AppError::ExternalServiceError(_) => "EXTERNAL_SERVICE_ERROR",
            AppError::InvalidFileData
            | AppError::FileSizeExceeded
            | AppError::InvalidFileName
            | AppError::UnsupportedFileExtension
            | AppError::MissingCredentials => "BAD_REQUEST",
            AppError::WrongCredentials | AppError::InvalidToken => "UNAUTHORIZED",
        };

        let body = axum::Json(ErrorResponse {
            code: code.to_string(),
            message: self.to_string(),
            details: None,
        });

        (status, body).into_response()
    }
}

impl From<JsonRejection> for AppError {
    fn from(rejection: JsonRejection) -> Self {
        Self::BadRequest(rejection.body_text())
    }
}

impl From<QueryRejection> for AppError {
    fn from(rejection: QueryRejection) -> Self {
        Self::BadRequest(rejection.body_text())
    }
}

/// handle_error is a function that middlewares the error handling in the application.
/// It takes a BoxError as input and returns an HTTP response.
/// It maps the error to an appropriate HTTP status code and constructs a JSON response body.
/// The function is used to handle errors that occur during the request processing.
/// It is designed to be used with the axum framework.
pub async fn handle_error(error: BoxError) -> impl IntoResponse {
    let status = if error.is::<tower::timeout::error::Elapsed>() {
        StatusCode::REQUEST_TIMEOUT
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    };

    let message = error.to_string();
    error!(?status, %message, "Request failed");

    let body = RestApiResponse::<()>::failure(status.as_u16(), message);

    (status, body)
}
