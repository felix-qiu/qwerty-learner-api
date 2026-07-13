use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Nullable<T>(pub Option<T>);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum PronunciationType {
    Us,
    Uk,
    Romaji,
    Zh,
    Ja,
    De,
    Hapin,
    Kk,
    Id,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum PhoneticType {
    Us,
    Uk,
    Romaji,
    Zh,
    Ja,
    De,
    Hapin,
    Kk,
    Id,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum WordDictationType {
    HideAll,
    HideVowel,
    HideConsonant,
    RandomHide,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum WordDictationOpenBy {
    User,
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(as = SoundResource)]
pub struct SoundResourceDto {
    #[schema(required = false)]
    pub key: String,
    #[schema(required = false)]
    pub name: String,
    #[schema(required = false)]
    pub filename: String,
    #[serde(flatten)]
    #[schema(ignore)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoopWordConfigDto {
    #[schema(required = false)]
    pub times: f64,
    #[serde(flatten)]
    #[schema(ignore)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct KeySoundsConfigDto {
    #[schema(required = false)]
    pub is_open: bool,
    #[schema(required = false)]
    pub is_open_click_sound: bool,
    #[schema(required = false)]
    pub volume: f64,
    #[schema(required = false, value_type = Option<SoundResourceDto>, nullable = true)]
    pub resource: Nullable<SoundResourceDto>,
    #[serde(flatten)]
    #[schema(ignore)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HintSoundsConfigDto {
    #[schema(required = false)]
    pub is_open: bool,
    #[schema(required = false)]
    pub volume: f64,
    #[schema(required = false)]
    pub is_open_wrong_sound: bool,
    #[schema(required = false)]
    pub is_open_correct_sound: bool,
    #[schema(required = false, value_type = Option<SoundResourceDto>, nullable = true)]
    pub wrong_resource: Nullable<SoundResourceDto>,
    #[schema(required = false, value_type = Option<SoundResourceDto>, nullable = true)]
    pub correct_resource: Nullable<SoundResourceDto>,
    #[serde(flatten)]
    #[schema(ignore)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PronunciationConfigDto {
    #[schema(required = false)]
    pub is_open: bool,
    #[schema(required = false)]
    pub volume: f64,
    #[schema(required = false)]
    pub r#type: PronunciationType,
    #[schema(required = false)]
    pub name: String,
    #[schema(required = false)]
    pub is_loop: bool,
    #[schema(required = false)]
    pub is_trans_read: bool,
    #[schema(required = false)]
    pub trans_volume: f64,
    #[schema(required = false)]
    pub rate: f64,
    #[serde(flatten)]
    #[schema(ignore)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FontSizeConfigDto {
    #[schema(required = false)]
    pub foreign_font: f64,
    #[schema(required = false)]
    pub translate_font: f64,
    #[serde(flatten)]
    #[schema(ignore)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RandomConfigDto {
    #[schema(required = false)]
    pub is_open: bool,
    #[serde(flatten)]
    #[schema(ignore)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PhoneticConfigDto {
    #[schema(required = false)]
    pub is_open: bool,
    #[schema(required = false)]
    pub r#type: PhoneticType,
    #[serde(flatten)]
    #[schema(ignore)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WordDictationConfigDto {
    #[schema(required = false)]
    pub is_open: bool,
    #[schema(required = false)]
    pub r#type: WordDictationType,
    #[schema(required = false)]
    pub open_by: WordDictationOpenBy,
    #[serde(flatten)]
    #[schema(ignore)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(as = UserSettings)]
pub struct UserSettingsDto {
    #[schema(required = false)]
    pub current_dict: String,
    #[schema(required = false)]
    pub current_chapter: i32,
    #[schema(required = false, inline)]
    pub loop_word_config: LoopWordConfigDto,
    #[schema(required = false, inline)]
    pub key_sounds_config: KeySoundsConfigDto,
    #[schema(required = false, inline)]
    pub hint_sounds_config: HintSoundsConfigDto,
    #[schema(required = false, inline)]
    pub pronunciation: PronunciationConfigDto,
    #[schema(required = false, inline)]
    pub fontsize: FontSizeConfigDto,
    #[schema(required = false, inline)]
    pub random_config: RandomConfigDto,
    #[schema(required = false, inline)]
    pub phonetic_config: PhoneticConfigDto,
    #[schema(required = false, inline)]
    pub word_dictation_config: WordDictationConfigDto,
    #[schema(required = false)]
    pub is_show_prev_and_next_word: bool,
    #[schema(required = false)]
    pub is_ignore_case: bool,
    #[schema(required = false)]
    pub is_show_answer_on_hover: bool,
    #[schema(required = false)]
    pub is_text_selectable: bool,
    #[schema(required = false)]
    pub is_open_dark_mode: bool,
    #[schema(required = false, value_type = Option<String>, format = DateTime, nullable = true)]
    pub dismiss_start_card_date: Nullable<String>,
    #[schema(required = false)]
    pub has_seen_enhanced_promotion: bool,
    #[serde(flatten)]
    #[schema(ignore)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
#[schema(as = UserSettingsPatch, description = "任意 UserSettings 子集，对象字段深合并")]
pub struct UserSettingsPatchDto(pub BTreeMap<String, Value>);
