use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
};

use async_trait::async_trait;
use chrono::{DateTime, Days, FixedOffset, NaiveDate, Utc};
use sqlx::PgPool;

use crate::{
    common::error::AppError,
    domains::{
        record::dto::record_dto::LetterMistakes,
        stats::{
            domain::{
                model::AnalysisWordRecord, repository::StatsRepository, service::StatsServiceTrait,
            },
            dto::stats_dto::{
                ActivityDayDto, AnalysisStatsDto, AnalysisStatsQuery, ChapterStatsDto,
                ChapterStatsQuery, DictStatsDto, StatsSummaryDto, WrongTimeRecordDto,
            },
            infra::impl_repository::StatsRepo,
        },
    },
};

const SHANGHAI_OFFSET_SECONDS: i32 = 8 * 60 * 60;

#[derive(Clone)]
pub struct StatsService {
    pool: PgPool,
    repo: Arc<dyn StatsRepository>,
}

#[derive(Default)]
struct DailyStats {
    exercise_count: i32,
    words: HashSet<String>,
    raw_word_count: i32,
    character_count: usize,
    total_timing_ms: f64,
    wrong_count: i32,
}

#[async_trait]
impl StatsServiceTrait for StatsService {
    fn create_service(pool: PgPool) -> Arc<dyn StatsServiceTrait> {
        Arc::new(Self {
            pool,
            repo: Arc::new(StatsRepo),
        })
    }

    async fn get_chapter_stats(
        &self,
        user_id: String,
        query: ChapterStatsQuery,
    ) -> Result<ChapterStatsDto, AppError> {
        let dict = required("dict", query.dict)?;
        validate_chapter(query.chapter)?;
        self.ensure_active_user(&user_id).await?;
        self.ensure_dictionary(&dict).await?;
        let row = self
            .repo
            .chapter_stats(&self.pool, &user_id, &dict, query.chapter)
            .await
            .map_err(db_error)?;

        Ok(ChapterStatsDto {
            dict,
            chapter: query.chapter,
            exercise_count: row.as_ref().map_or(0, |row| row.exercise_count),
            avg_wrong_word_count: row.as_ref().map_or(0.0, |row| row.avg_wrong_word_count),
            avg_wrong_input_count: row.map_or(0.0, |row| row.avg_wrong_input_count),
        })
    }

    async fn get_dictionary_stats(
        &self,
        user_id: String,
        dict_id: String,
    ) -> Result<DictStatsDto, AppError> {
        let dict = required("dictId", dict_id)?;
        self.ensure_active_user(&user_id).await?;
        let chapter_count = self.ensure_dictionary(&dict).await?;
        let exercised_chapter_count = self
            .repo
            .exercised_chapter_count(&self.pool, &user_id, &dict)
            .await
            .map_err(db_error)?;

        Ok(DictStatsDto {
            dict,
            exercised_chapter_count,
            chapter_count,
        })
    }

    async fn get_analysis_stats(
        &self,
        user_id: String,
        query: AnalysisStatsQuery,
    ) -> Result<AnalysisStatsDto, AppError> {
        validate_explicit_range(query.from, query.to)?;
        self.ensure_active_user(&user_id).await?;
        let first_record_at = self
            .repo
            .first_word_record_at(&self.pool, &user_id)
            .await
            .map_err(db_error)?;
        let Some(first_record_at) = first_record_at else {
            return Ok(AnalysisStatsDto::empty());
        };
        let from = query.from.unwrap_or(first_record_at);
        let to = query.to.unwrap_or_else(|| Utc::now().timestamp());
        let (start_date, end_date) = validate_range(from, to)?;
        let records = self
            .repo
            .analysis_records(&self.pool, &user_id, from, to)
            .await
            .map_err(db_error)?;
        if records.is_empty() {
            return Ok(AnalysisStatsDto::empty());
        }

        aggregate_analysis(records, start_date, end_date)
    }

    async fn get_stats_summary(&self, user_id: String) -> Result<StatsSummaryDto, AppError> {
        self.ensure_active_user(&user_id).await?;
        let row = self
            .repo
            .summary(&self.pool, &user_id)
            .await
            .map_err(db_error)?
            .ok_or(AppError::InternalError)?;
        Ok(StatsSummaryDto {
            word_record_count: row.word_record_count,
            chapter_record_count: row.chapter_record_count,
            total_time_seconds: row.total_time_seconds,
            first_practice_at: row.first_practice_at,
        })
    }
}

impl StatsService {
    async fn ensure_active_user(&self, user_id: &str) -> Result<(), AppError> {
        if self
            .repo
            .is_active_user(&self.pool, user_id)
            .await
            .map_err(db_error)?
        {
            Ok(())
        } else {
            Err(AppError::InvalidToken)
        }
    }

    async fn ensure_dictionary(&self, dict_id: &str) -> Result<i32, AppError> {
        self.repo
            .published_dictionary_chapter_count(&self.pool, dict_id)
            .await
            .map_err(db_error)?
            .ok_or_else(|| AppError::NotFound("Dictionary not found".into()))
    }
}

fn aggregate_analysis(
    records: Vec<AnalysisWordRecord>,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<AnalysisStatsDto, AppError> {
    let mut daily = BTreeMap::<NaiveDate, DailyStats>::new();
    let mut date = start_date;
    loop {
        daily.insert(date, DailyStats::default());
        if date == end_date {
            break;
        }
        date = date.checked_add_days(Days::new(1)).ok_or_else(|| {
            AppError::ValidationError("from and to produce an invalid date range".into())
        })?;
    }

    let mut wrong_keys = HashMap::<String, i32>::new();
    for record in records {
        let date = shanghai_date(record.time_stamp)?;
        let stats = daily.get_mut(&date).ok_or(AppError::InternalError)?;
        stats.exercise_count = stats
            .exercise_count
            .checked_add(1)
            .ok_or(AppError::InternalError)?;
        stats.raw_word_count = stats
            .raw_word_count
            .checked_add(1)
            .ok_or(AppError::InternalError)?;
        stats.character_count = stats
            .character_count
            .checked_add(record.word.encode_utf16().count())
            .ok_or(AppError::InternalError)?;
        stats.words.insert(record.word);
        stats.total_timing_ms += record.timing.iter().sum::<f64>();
        stats.wrong_count = stats
            .wrong_count
            .checked_add(record.wrong_count)
            .ok_or(AppError::InternalError)?;

        let mistakes = serde_json::from_value::<LetterMistakes>(record.mistakes)
            .map_err(|_| AppError::InternalError)?;
        for key in mistakes.0.into_values().flatten() {
            let count = wrong_keys.entry(key.to_uppercase()).or_default();
            *count = count.checked_add(1).ok_or(AppError::InternalError)?;
        }
    }

    let mut exercise_record = Vec::with_capacity(daily.len());
    let mut word_record = Vec::with_capacity(daily.len());
    let mut wpm_record = Vec::new();
    let mut accuracy_record = Vec::new();
    for (date, stats) in daily {
        let date = date.format("%Y-%m-%d").to_string();
        let unique_word_count =
            i32::try_from(stats.words.len()).map_err(|_| AppError::InternalError)?;
        exercise_record.push(ActivityDayDto {
            date: date.clone(),
            count: stats.exercise_count,
            level: activity_level(stats.exercise_count),
        });
        word_record.push(ActivityDayDto {
            date: date.clone(),
            count: unique_word_count,
            level: activity_level(unique_word_count),
        });
        if stats.total_timing_ms > 0.0 {
            let wpm =
                ((f64::from(stats.raw_word_count) * 60_000.0) / stats.total_timing_ms).round();
            if wpm.is_finite() && wpm > 0.0 && wpm <= f64::from(i32::MAX) {
                wpm_record.push((date.clone(), wpm as i32));
            }
        }
        let denominator = stats
            .character_count
            .checked_add(usize::try_from(stats.wrong_count).map_err(|_| AppError::InternalError)?)
            .ok_or(AppError::InternalError)?;
        if denominator > 0 {
            let accuracy = ((stats.character_count as f64 / denominator as f64) * 100.0).round();
            if accuracy > 0.0 {
                accuracy_record.push((date, accuracy as i32));
            }
        }
    }

    let mut wrong_time_record = wrong_keys
        .into_iter()
        .map(|(name, value)| WrongTimeRecordDto { name, value })
        .collect::<Vec<_>>();
    wrong_time_record.sort_by(|left, right| {
        right
            .value
            .cmp(&left.value)
            .then_with(|| left.name.cmp(&right.name))
    });

    Ok(AnalysisStatsDto {
        is_empty: false,
        exercise_record,
        word_record,
        wpm_record,
        accuracy_record,
        wrong_time_record,
    })
}

fn validate_range(from: i64, to: i64) -> Result<(NaiveDate, NaiveDate), AppError> {
    if from > to {
        return Err(AppError::ValidationError(
            "from must be less than or equal to to".into(),
        ));
    }
    let start = shanghai_date(from)?;
    let end = shanghai_date(to)?;
    Ok((start, end))
}

fn validate_explicit_range(from: Option<i64>, to: Option<i64>) -> Result<(), AppError> {
    if let Some(from) = from {
        shanghai_date(from)?;
    }
    if let Some(to) = to {
        shanghai_date(to)?;
    }
    if matches!((from, to), (Some(from), Some(to)) if from > to) {
        return Err(AppError::ValidationError(
            "from must be less than or equal to to".into(),
        ));
    }
    Ok(())
}

fn shanghai_date(timestamp: i64) -> Result<NaiveDate, AppError> {
    let datetime = DateTime::from_timestamp(timestamp, 0)
        .ok_or_else(|| AppError::ValidationError("invalid Unix timestamp".into()))?;
    let offset = FixedOffset::east_opt(SHANGHAI_OFFSET_SECONDS).ok_or(AppError::InternalError)?;
    Ok(datetime.with_timezone(&offset).date_naive())
}

fn activity_level(count: i32) -> i32 {
    match count {
        ..=0 => 0,
        1..=3 => 1,
        4..=7 => 2,
        8..=11 => 3,
        _ => 4,
    }
}

fn validate_chapter(chapter: i32) -> Result<(), AppError> {
    if chapter < -1 {
        Err(AppError::ValidationError(
            "chapter must be -1 or greater than or equal to 0".into(),
        ))
    } else {
        Ok(())
    }
}

fn required(field: &str, value: String) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        Err(AppError::ValidationError(format!(
            "{field} must not be empty"
        )))
    } else {
        Ok(value.to_string())
    }
}

fn db_error(error: sqlx::Error) -> AppError {
    tracing::error!(?error, "stats database operation failed");
    AppError::InternalError
}

#[cfg(test)]
mod tests {
    use crate::domains::stats::domain::model::AnalysisWordRecord;

    use super::{
        activity_level, aggregate_analysis, shanghai_date, validate_chapter,
        validate_explicit_range, validate_range,
    };

    #[test]
    fn uses_shanghai_natural_days() {
        assert_eq!(
            shanghai_date(1_704_038_400).unwrap().to_string(),
            "2024-01-01"
        );
        assert!(validate_range(2, 1).is_err());
        assert!(validate_explicit_range(Some(2), Some(1)).is_err());
        assert!(validate_explicit_range(Some(i64::MAX), None).is_err());
        assert!(validate_chapter(-2).is_err());
        assert!(validate_chapter(-1).is_ok());
        assert!(validate_chapter(0).is_ok());
    }

    #[test]
    fn calculates_activity_levels() {
        assert_eq!(activity_level(0), 0);
        assert_eq!(activity_level(3), 1);
        assert_eq!(activity_level(4), 2);
        assert_eq!(activity_level(8), 3);
        assert_eq!(activity_level(12), 4);
    }

    #[test]
    fn aggregates_analysis_by_shanghai_day() {
        let records = vec![
            AnalysisWordRecord {
                word: "ab".into(),
                timing: vec![60_000.0],
                wrong_count: 1,
                mistakes: serde_json::json!({"0": ["a"]}),
                time_stamp: 1_751_385_540,
            },
            AnalysisWordRecord {
                word: "bb".into(),
                timing: vec![30_000.0],
                wrong_count: 1,
                mistakes: serde_json::json!({"0": ["e"]}),
                time_stamp: 1_751_385_660,
            },
            AnalysisWordRecord {
                word: "bb".into(),
                timing: vec![30_000.0],
                wrong_count: 0,
                mistakes: serde_json::json!({}),
                time_stamp: 1_751_385_720,
            },
        ];
        let (start, end) = validate_range(1_751_385_540, 1_751_385_720).unwrap();
        let stats = aggregate_analysis(records, start, end).unwrap();

        assert!(!stats.is_empty);
        assert_eq!(stats.exercise_record[0].date, "2025-07-01");
        assert_eq!(stats.exercise_record[0].count, 1);
        assert_eq!(stats.exercise_record[1].date, "2025-07-02");
        assert_eq!(stats.exercise_record[1].count, 2);
        assert_eq!(stats.word_record[1].count, 1);
        assert_eq!(
            stats.wpm_record,
            vec![("2025-07-01".into(), 1), ("2025-07-02".into(), 2)]
        );
        assert_eq!(
            stats.accuracy_record,
            vec![("2025-07-01".into(), 67), ("2025-07-02".into(), 80)]
        );
        assert_eq!(stats.wrong_time_record[0].name, "A");
        assert_eq!(stats.wrong_time_record[1].name, "E");
    }

    #[test]
    fn fills_dates_without_records() {
        let records = vec![
            AnalysisWordRecord {
                word: "a".into(),
                timing: vec![60_000.0],
                wrong_count: 0,
                mistakes: serde_json::json!({}),
                time_stamp: 1_751_385_660,
            },
            AnalysisWordRecord {
                word: "b".into(),
                timing: vec![60_000.0],
                wrong_count: 0,
                mistakes: serde_json::json!({}),
                time_stamp: 1_751_558_460,
            },
        ];
        let (start, end) = validate_range(1_751_385_660, 1_751_558_460).unwrap();
        let stats = aggregate_analysis(records, start, end).unwrap();

        assert_eq!(stats.exercise_record.len(), 3);
        assert_eq!(stats.exercise_record[1].date, "2025-07-03");
        assert_eq!(stats.exercise_record[1].count, 0);
        assert_eq!(stats.exercise_record[1].level, 0);
        assert_eq!(stats.word_record[1].count, 0);
        assert!(!stats
            .wpm_record
            .iter()
            .any(|(date, _)| date == "2025-07-03"));
        assert!(!stats
            .accuracy_record
            .iter()
            .any(|(date, _)| date == "2025-07-03"));
    }
}
