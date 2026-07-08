use axum::{
    body::Body,
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};

use crate::{
    common::{app_state::AppState, error::AppError},
    domains::image::dto::image_dto::GenerateWordImageRequest,
};

#[utoipa::path(
    post,
    path = "/image/word",
    request_body = GenerateWordImageRequest,
    responses(
        (status = 200, description = "Generated 1024x1024 WebP image", content_type = "image/webp")
    ),
    tag = "Images"
)]
pub async fn generate_word_image(
    State(state): State<AppState>,
    Json(payload): Json<GenerateWordImageRequest>,
) -> Result<impl IntoResponse, AppError> {
    let image = state
        .image_service
        .generate_word_image(payload.word)
        .await?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, image.content_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"{}\"", image.file_name),
        )
        .header(header::CACHE_CONTROL, "no-store")
        .header("X-Image-Width", "1024")
        .header("X-Image-Height", "1024")
        .header("X-Image-Display-Width", "320")
        .header("X-Image-Display-Height", "320")
        .body(Body::from(image.bytes))
        .map_err(|err| {
            tracing::error!("Error building generated image response: {err}");
            AppError::InternalError
        })?;

    Ok(response)
}
