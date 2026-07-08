use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{
    common::{config::Config, error::AppError},
    domains::image::{domain::service::ImageServiceTrait, dto::image_dto::GeneratedImageDto},
};

const WORD_IMAGE_PROMPT_PREFIX: &str = "A cute children's English learning illustration of";
const WORD_IMAGE_PROMPT_SUFFIX: &str = "\
Kindergarten textbook style.
Simple cartoon style.
One clear main subject.
Centered composition.
White or very light plain background.
Bright and friendly colors.
Clean shapes, soft outlines.
Easy for children aged 3-8 to recognize.
No text, no watermark, no extra clutter.";

#[derive(Clone)]
pub struct ImageService {
    config: Config,
    client: Client,
}

#[derive(Debug, Serialize)]
struct XinferenceImageRequest {
    model: String,
    prompt: String,
    n: u8,
    size: String,
    response_format: String,
    output_format: String,
    num_inference_steps: u32,
    guidance_scale: f32,
}

#[derive(Debug, Deserialize)]
struct XinferenceImageResponse {
    data: Vec<XinferenceImageData>,
}

#[derive(Debug, Deserialize)]
struct XinferenceImageData {
    b64_json: Option<String>,
    url: Option<String>,
}

#[async_trait]
impl ImageServiceTrait for ImageService {
    fn create_service(config: Config) -> Arc<dyn ImageServiceTrait> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.xinference_request_timeout_secs))
            .build()
            .expect("failed to build reqwest client");

        Arc::new(Self { config, client })
    }

    async fn generate_word_image(&self, word: String) -> Result<GeneratedImageDto, AppError> {
        let word = normalize_word(&word)?;
        let prompt = build_word_image_prompt(&word);

        let request = XinferenceImageRequest {
            model: self.config.xinference_image_model.clone(),
            prompt,
            n: 1,
            size: self.config.xinference_image_size.clone(),
            response_format: "b64_json".to_string(),
            output_format: "webp".to_string(),
            num_inference_steps: self.config.xinference_image_steps,
            guidance_scale: self.config.xinference_image_guidance_scale,
        };

        let endpoint = format!(
            "{}/v1/images/generations",
            self.config.xinference_base_url.trim_end_matches('/')
        );

        let response = self
            .client
            .post(endpoint)
            .json(&request)
            .send()
            .await
            .map_err(|err| {
                tracing::error!("Xinference image request failed: {err}");
                AppError::ExternalServiceError("Xinference image request failed".into())
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Xinference image request returned {status}: {body}");
            return Err(AppError::ExternalServiceError(format!(
                "Xinference image request returned {status}"
            )));
        }

        let payload = response
            .json::<XinferenceImageResponse>()
            .await
            .map_err(|err| {
                tracing::error!("Failed to parse Xinference image response: {err}");
                AppError::ExternalServiceError("Invalid Xinference image response".into())
            })?;

        let data =
            payload.data.into_iter().next().ok_or_else(|| {
                AppError::ExternalServiceError("Xinference returned no image".into())
            })?;

        let image_bytes = match (data.b64_json, data.url) {
            (Some(b64_json), _) => decode_image_base64(&b64_json)?,
            (None, Some(url)) => self.download_image(&url).await?,
            (None, None) => {
                return Err(AppError::ExternalServiceError(
                    "Xinference returned no image payload".into(),
                ))
            }
        };
        let image_format = detect_image_format(&image_bytes)?;

        Ok(GeneratedImageDto {
            bytes: image_bytes,
            file_name: format!("{word}.{}", image_format.extension),
            content_type: image_format.content_type.to_string(),
        })
    }
}

impl ImageService {
    async fn download_image(&self, url: &str) -> Result<Vec<u8>, AppError> {
        let url = if url.starts_with('/') {
            format!(
                "{}{}",
                self.config.xinference_base_url.trim_end_matches('/'),
                url
            )
        } else {
            url.to_string()
        };

        let response = self.client.get(url).send().await.map_err(|err| {
            tracing::error!("Failed to download Xinference image URL: {err}");
            AppError::ExternalServiceError("Failed to download generated image".into())
        })?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(AppError::ExternalServiceError(format!(
                "Generated image download returned {status}"
            )));
        }

        response
            .bytes()
            .await
            .map(|bytes| bytes.to_vec())
            .map_err(|err| {
                tracing::error!("Failed to read generated image bytes: {err}");
                AppError::ExternalServiceError("Failed to read generated image".into())
            })
    }
}

fn normalize_word(word: &str) -> Result<String, AppError> {
    let word = word.trim();
    if word.is_empty() {
        return Err(AppError::ValidationError("word is required".into()));
    }

    if word.len() > 64 {
        return Err(AppError::ValidationError(
            "word must be 64 characters or fewer".into(),
        ));
    }

    let valid = word
        .chars()
        .all(|ch| ch.is_ascii_alphabetic() || ch == ' ' || ch == '-' || ch == '\'');

    if !valid {
        return Err(AppError::ValidationError(
            "word must contain only English letters, spaces, hyphens, or apostrophes".into(),
        ));
    }

    Ok(word.to_string())
}

fn build_word_image_prompt(word: &str) -> String {
    format!("{WORD_IMAGE_PROMPT_PREFIX} {word}.\n{WORD_IMAGE_PROMPT_SUFFIX}")
}

fn decode_image_base64(value: &str) -> Result<Vec<u8>, AppError> {
    let encoded = value
        .split_once(',')
        .map(|(_, encoded)| encoded)
        .unwrap_or(value);

    general_purpose::STANDARD.decode(encoded).map_err(|err| {
        tracing::error!("Failed to decode generated image base64: {err}");
        AppError::ExternalServiceError("Invalid generated image data".into())
    })
}

struct ImageFormatInfo {
    content_type: &'static str,
    extension: &'static str,
}

fn detect_image_format(bytes: &[u8]) -> Result<ImageFormatInfo, AppError> {
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Ok(ImageFormatInfo {
            content_type: "image/webp",
            extension: "webp",
        });
    }

    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Ok(ImageFormatInfo {
            content_type: "image/png",
            extension: "png",
        });
    }

    if bytes.starts_with(b"\xff\xd8\xff") {
        return Ok(ImageFormatInfo {
            content_type: "image/jpeg",
            extension: "jpg",
        });
    }

    Err(AppError::ExternalServiceError(
        "Xinference did not return a supported image".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::{build_word_image_prompt, detect_image_format, normalize_word};

    #[test]
    fn prompt_uses_required_word_image_template() {
        let prompt = build_word_image_prompt("apple");

        assert!(prompt.contains("A cute children's English learning illustration of apple."));
        assert!(prompt.contains("Kindergarten textbook style."));
        assert!(prompt.contains("No text, no watermark, no extra clutter."));
    }

    #[test]
    fn normalize_word_rejects_non_english_input() {
        assert!(normalize_word("apple").is_ok());
        assert!(normalize_word("ice cream").is_ok());
        assert!(normalize_word("<script>").is_err());
        assert!(normalize_word("苹果").is_err());
    }

    #[test]
    fn detect_image_format_accepts_common_image_types() {
        assert_eq!(
            detect_image_format(b"RIFFxxxxWEBPmore-data")
                .unwrap()
                .content_type,
            "image/webp"
        );
        assert_eq!(
            detect_image_format(b"\x89PNG\r\n\x1a\nmore-data")
                .unwrap()
                .content_type,
            "image/png"
        );
        assert_eq!(
            detect_image_format(b"\xff\xd8\xffmore-data")
                .unwrap()
                .content_type,
            "image/jpeg"
        );
    }
}
