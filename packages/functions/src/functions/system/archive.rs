//! User archive (save slot) business logic — mirrors Java `SysUserArchiveService`.

use anyhow::{Result, anyhow};
use chrono::Utc;
use sea_orm::{
    ActiveValue::{NotSet, Set},
    QueryFilter, QueryOrder,
    prelude::*,
};

use _database::{DB_CONN, models::system::sys_user_archive as archive_model};
use _utils::{
    db_operations::SafeEntityTrait,
    jwt::AuthInfo,
    models::{ArchiveSlotVo, wrapper::CommonResponse},
};

/// 把实体行转换成槽位 VO：`data` 列存的是存档 JSON 文本，直接作为 `archive` 字段；
/// 兼容历史数据（data 为非字符串 JSON 值）时退化为重新序列化。
fn entity_to_slot_vo(a: archive_model::Model) -> ArchiveSlotVo {
    ArchiveSlotVo {
        slot_index: a.slot_index,
        time: a.create_time.and_utc().timestamp_millis() as f64,
        archive: a
            .data
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|| serde_json::to_string(&a.data).unwrap_or_default()),
    }
}

/// 从任意 JSON body 中提取存档文本：
/// 前端直接上传 `JSON.stringify({Data_KYJG, Time_KYJG, Preference})` 的存档体，
/// 若带有 `archive` 字段（`{time, archive, historyIndex}` 包装体）则取该字段，
/// 否则把整个请求体序列化作为存档文本。
fn extract_archive(body: &serde_json::Value) -> String {
    body.get("archive")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| serde_json::to_string(body).unwrap_or_default())
}

/// 从任意 JSON body 中提取保存时间（毫秒时间戳），取不到默认当前时间。
fn extract_time(body: &serde_json::Value) -> chrono::NaiveDateTime {
    body.get("time")
        .and_then(|v| v.as_f64())
        .and_then(|ms| chrono::DateTime::from_timestamp_millis(ms as i64))
        .map(|dt| dt.naive_utc())
        .unwrap_or_else(|| Utc::now().naive_utc())
}

/// Get the latest archive for a given slot index.
pub async fn do_get_last(
    _auth: AuthInfo,
    user_id: i64,
    slot_index: i32,
) -> Result<CommonResponse<Option<ArchiveSlotVo>>> {
    let db = &DB_CONN.wait().pg_conn;
    let archive = archive_model::Entity::find_safety()
        .filter(archive_model::Column::UserId.eq(user_id))
        .filter(archive_model::Column::SlotIndex.eq(slot_index))
        .order_by_desc(archive_model::Column::CreateTime)
        .one(db)
        .await?;
    Ok(CommonResponse::new(Ok(archive.map(entity_to_slot_vo))))
}

/// Get all history archives for a given slot index.
pub async fn do_get_history(
    _auth: AuthInfo,
    user_id: i64,
    slot_index: i32,
) -> Result<CommonResponse<Vec<serde_json::Value>>> {
    let db = &DB_CONN.wait().pg_conn;
    let items = archive_model::Entity::find_safety()
        .filter(archive_model::Column::UserId.eq(user_id))
        .filter(archive_model::Column::SlotIndex.eq(slot_index))
        .order_by_desc(archive_model::Column::CreateTime)
        .all(db)
        .await?;
    let record: Vec<serde_json::Value> = items
        .into_iter()
        .map(|a| serde_json::to_value(entity_to_slot_vo(a)).unwrap_or_default())
        .collect();
    Ok(CommonResponse::new(Ok(record)))
}

/// Get all history archives across all slots for the user.
pub async fn do_get_all_history(
    _auth: AuthInfo,
    user_id: i64,
) -> Result<CommonResponse<Vec<serde_json::Value>>> {
    let db = &DB_CONN.wait().pg_conn;
    let items = archive_model::Entity::find_safety()
        .filter(archive_model::Column::UserId.eq(user_id))
        .order_by_desc(archive_model::Column::CreateTime)
        .all(db)
        .await?;
    let record: Vec<serde_json::Value> = items
        .into_iter()
        .map(|a| serde_json::to_value(entity_to_slot_vo(a)).unwrap_or_default())
        .collect();
    Ok(CommonResponse::new(Ok(record)))
}

/// Save (put) an archive to a slot.
/// 请求体为任意 JSON：前端直接上传存档 JSON 字符串；兼容 `{time, archive, historyIndex}` 包装体。
pub async fn do_save(
    _auth: AuthInfo,
    user_id: i64,
    slot_index: i32,
    name: Option<String>,
    body: serde_json::Value,
) -> Result<CommonResponse<serde_json::Value>> {
    let db = &DB_CONN.wait().pg_conn;
    let archive = extract_archive(&body);
    let now = extract_time(&body);

    let am = archive_model::ActiveModel {
        version: Set(0),
        id: NotSet,
        create_time: Set(now),
        update_time: Set(None),
        creator_id: Set(Some(user_id)),
        updater_id: Set(None),
        del_flag: Set(false),
        name: Set(name),
        slot_index: Set(slot_index),
        user_id: Set(user_id),
        data: Set(serde_json::Value::String(archive)),
    };
    let res = archive_model::Entity::insert(am).exec(db).await?;
    Ok(CommonResponse::new(Ok(serde_json::json!({
        "id": res.last_insert_id
    }))))
}

/// Rename an archive slot.
pub async fn do_rename(_auth: AuthInfo, id: i64, name: String) -> Result<CommonResponse<()>> {
    let db = &DB_CONN.wait().pg_conn;
    let a = archive_model::Entity::find_safety_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Archive not found"))?;
    let mut am: archive_model::ActiveModel = a.into();
    am.name = Set(Some(name));
    archive_model::Entity::update_safety(am)?.exec(db).await?;
    Ok(CommonResponse::new(Ok(())))
}

/// Rename an archive slot (renames the latest archive in the slot).
pub async fn do_rename_by_slot(
    user_id: i64,
    slot_index: i32,
    new_name: String,
) -> Result<CommonResponse<()>> {
    let db = &DB_CONN.wait().pg_conn;
    let a = archive_model::Entity::find_safety()
        .filter(archive_model::Column::UserId.eq(user_id))
        .filter(archive_model::Column::SlotIndex.eq(slot_index))
        .order_by_desc(archive_model::Column::CreateTime)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Archive not found"))?;
    let mut am: archive_model::ActiveModel = a.into();
    am.name = Set(Some(new_name));
    archive_model::Entity::update_safety(am)?.exec(db).await?;
    Ok(CommonResponse::new(Ok(())))
}

/// Restore from an archive (return the archived data).
pub async fn do_restore(_auth: AuthInfo, id: i64) -> Result<CommonResponse<serde_json::Value>> {
    let db = &DB_CONN.wait().pg_conn;
    let a = archive_model::Entity::find_safety_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Archive not found"))?;
    Ok(CommonResponse::new(Ok(a.data)))
}

/// Delete an archive slot (soft-delete every archive in the slot).
pub async fn do_delete_slot(user_id: i64, slot_index: i32) -> Result<CommonResponse<()>> {
    let db = &DB_CONN.wait().pg_conn;
    // 批量软删（避免逐条 find+delete 的 N+1 往返）
    archive_model::Entity::update_many()
        .col_expr(
            archive_model::Column::DelFlag,
            sea_orm::sea_query::Expr::value(true),
        )
        .filter(archive_model::Column::UserId.eq(user_id))
        .filter(archive_model::Column::SlotIndex.eq(slot_index))
        .exec(db)
        .await?;
    Ok(CommonResponse::new(Ok(())))
}

/// Delete an archive slot (soft delete).
pub async fn do_delete(_auth: AuthInfo, id: i64) -> Result<CommonResponse<()>> {
    let db = &DB_CONN.wait().pg_conn;
    let a = archive_model::Entity::find_safety_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Archive not found"))?;
    archive_model::Entity::delete_safety(a.into())?
        .exec(db)
        .await?;
    Ok(CommonResponse::new(Ok(())))
}
