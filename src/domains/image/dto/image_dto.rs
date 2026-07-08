use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct GenerateWordImageRequest {
    #[schema(example = "apple")]
    pub word: String,
}

pub struct GeneratedImageDto {
    pub bytes: Vec<u8>,
    pub file_name: String,
    pub content_type: String,
}
