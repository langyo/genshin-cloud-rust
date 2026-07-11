use anyhow::Result;
use sea_orm::{QueryFilter, QuerySelect, prelude::*};

use _database::{DB_CONN, models::common::score_stat as score_stat_model};
use _utils::{
    db_operations::SafeEntityTrait,
    jwt::AuthInfo,
    models::score::{ScoreDataRequest, ScoreGenerateRequest, ScoreResponse, ScoreSample},
    models::wrapper::CommonResponse,
};

/// 生成评分统计数据。
///
/// Java 侧 `ScoreGenerateService` 是一个复杂的批处理：扫描 punctuate 记录、
/// 按 scope/span 桶式聚合成 ScoreStat 行写入 score_stat 表。这个批处理管线
/// 尚未移植；当前返回空结果而不是伪造数据（旧实现用 LCG 随机数伪装真实评分，
/// 对客户端有误导性）。调用方应先触发 generate（写入 score_stat），再调
/// do_get_score_data 读取。
pub async fn do_generate_score(
    _auth: AuthInfo,
    _payload: ScoreGenerateRequest,
) -> Result<CommonResponse<ScoreResponse>> {
    // TODO(dev): 移植 Java 的 ScoreGenerateService 批处理（扫描 punctuate →
    // 按 scope/span 聚合 → 写入 score_stat 表）。当前为空响应。
    Ok(CommonResponse::new(Ok(ScoreResponse {
        samples: Vec::new(),
        average: 0.0,
    })))
}

/// 读取评分统计数据——从 score_stat 表查询真实聚合记录。
///
/// 按 scope + span + 时间范围过滤，返回每个统计周期的评分样本。
pub async fn do_get_score_data(
    _auth: AuthInfo,
    payload: ScoreDataRequest,
) -> Result<CommonResponse<ScoreResponse>> {
    let db = &DB_CONN.wait().pg_conn;

    // 将毫秒时间戳转换为 NaiveDateTime 用于查询
    let start = chrono::DateTime::from_timestamp_millis(payload.start_time as i64)
        .map(|dt| dt.naive_utc())
        .unwrap_or_else(|| chrono::NaiveDateTime::MIN);
    let end = chrono::DateTime::from_timestamp_millis(payload.end_time as i64)
        .map(|dt| dt.naive_utc())
        .unwrap_or_else(|| chrono::Utc::now().naive_utc());

    let query = score_stat_model::Entity::find_safety()
        .filter(score_stat_model::Column::Scope.eq(&payload.scope))
        .filter(score_stat_model::Column::Span.eq(&payload.span))
        .filter(score_stat_model::Column::SpanStartTime.gte(start))
        .filter(score_stat_model::Column::SpanEndTime.lte(end))
        .limit(10_000);

    let stats = query.all(db).await?;

    let samples: Vec<ScoreSample> = stats
        .iter()
        .map(|s| ScoreSample {
            time: s.span_end_time.and_utc().timestamp_millis() as f64,
            // score_stat.content 存的是 ScopeStatType 枚举；真实评分值需要从
            // generate 管线写入后才能填充。当前返回 0.0（占位，非伪造随机数）。
            score: 0.0,
        })
        .collect();

    let average = if samples.is_empty() {
        0.0
    } else {
        samples.iter().map(|s| s.score).sum::<f64>() / samples.len() as f64
    };

    Ok(CommonResponse::new(Ok(ScoreResponse { samples, average })))
}
