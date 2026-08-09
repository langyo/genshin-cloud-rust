use anyhow::{Result, anyhow};
use chrono::Utc;

use sea_orm::{
    ActiveValue::{NotSet, Set},
    QueryFilter, QuerySelect,
    prelude::*,
};

use _database::DB_CONN;
use _database::models::system::sys_user as sys_user_model;
use _utils::{
    db_operations::SafeEntityTrait,
    jwt::AuthInfo,
    models::{CommonResponse, Pagination, SysUserVO},
    types::{AccessPolicyItemEnum, SystemUserRole, UserSort},
};

// 业务处理函数
pub async fn do_register(
    _auth: AuthInfo,
    access_policy: Option<Vec<AccessPolicyItemEnum>>,
    logo: Option<String>,
    remark: Option<String>,
    role_id: Option<SystemUserRole>,
    username: String,
    password: String,
) -> Result<CommonResponse<i64>> {
    let db = &DB_CONN.wait().pg_conn;

    let now = Utc::now().naive_utc();
    let am = sys_user_model::ActiveModel {
        version: Set(0),
        id: NotSet,
        create_time: Set(now),
        update_time: Set(None),
        creator_id: Set(None),
        updater_id: Set(None),
        del_flag: Set(false),

        username: Set(username),
        password: Set(_utils::bcrypt::generate_storage_password(&password)?),
        nickname: Set(None),
        qq: Set(None),
        phone: Set(None),
        logo: Set(logo),
        role_id: Set(role_id.unwrap_or(SystemUserRole::MapUser)),
        access_policy: Set(access_policy.map(_utils::types::AccessPolicyList)),
        remark: Set(remark),
    };

    let res = sys_user_model::Entity::insert(am).exec(db).await?;
    Ok(CommonResponse::new(Ok(res.last_insert_id)))
}

pub async fn do_register_qq(
    access_policy: Option<Vec<AccessPolicyItemEnum>>,
    logo: Option<String>,
    remark: Option<String>,
    username: String,
    password: String,
    qq: Option<String>,
) -> Result<CommonResponse<i64>> {
    let db = &DB_CONN.wait().pg_conn;

    let now = Utc::now().naive_utc();
    let am = sys_user_model::ActiveModel {
        version: Set(0),
        id: NotSet,
        create_time: Set(now),
        update_time: Set(None),
        creator_id: Set(None),
        updater_id: Set(None),
        del_flag: Set(false),

        username: Set(username),
        password: Set(_utils::bcrypt::generate_storage_password(&password)?),
        nickname: Set(None),
        qq: Set(qq),
        phone: Set(None),
        logo: Set(logo),
        // 公开接口：不信任客户端传入的角色，固定注册为地图用户
        role_id: Set(SystemUserRole::MapUser),
        access_policy: Set(access_policy.map(_utils::types::AccessPolicyList)),
        remark: Set(remark),
    };

    let res = sys_user_model::Entity::insert(am).exec(db).await?;
    Ok(CommonResponse::new(Ok(res.last_insert_id)))
}

pub async fn do_get_info(auth: AuthInfo, user_id: i64) -> Result<SysUserVO> {
    let db = &DB_CONN.wait().pg_conn;
    let m = sys_user_model::Entity::find_safety_by_id(user_id)
        .one(db)
        .await?;
    let m = m.ok_or(anyhow!("User not found"))?;

    // 权限校验：本人可读全部字段；非本人需 MapManager 及以上（role_id <= MapManager，
    // Admin=0 / MapManager=1），否则只返回公开字段，不下发 qq/phone/accessPolicy。
    let is_self = auth.info.id == user_id;
    let is_manager = (auth.info.role_id as i32) <= (SystemUserRole::MapManager as i32);
    if !is_self && !is_manager {
        return Ok(SysUserVO {
            id: m.id,
            username: m.username,
            nickname: m.nickname,
            qq: None,
            phone: None,
            logo: m.logo,
            role_id: m.role_id,
            access_policy: Default::default(),
            remark: m.remark,
        });
    }
    Ok(m.into())
}

#[allow(clippy::too_many_arguments)]
pub async fn do_update(
    auth: AuthInfo,
    id: i64,
    access_policy: Option<Vec<AccessPolicyItemEnum>>,
    logo: Option<String>,
    nickname: Option<String>,
    phone: Option<String>,
    qq: Option<String>,
    remark: Option<String>,
    role_id: Option<SystemUserRole>,
) -> Result<CommonResponse<()>> {
    let db = &DB_CONN.wait().pg_conn;
    // 权限校验：仅 Admin 可修改他人资料；普通用户只能改自己。
    let is_admin = auth.info.role_id == SystemUserRole::Admin;
    if !is_admin && auth.info.id != id {
        return Err(anyhow!("Forbidden: only admins can update other users"));
    }
    let m = sys_user_model::Entity::find_safety_by_id(id)
        .one(db)
        .await?;
    let m = m.ok_or(anyhow!("User not found"))?;
    let mut am: sys_user_model::ActiveModel = m.into();

    if let Some(ap) = access_policy {
        am.access_policy = Set(Some(_utils::types::AccessPolicyList(ap)));
    }
    if let Some(l) = logo {
        am.logo = Set(Some(l));
    }
    if let Some(n) = nickname {
        am.nickname = Set(Some(n));
    }
    if let Some(p) = phone {
        am.phone = Set(Some(p));
    }
    if let Some(q) = qq {
        am.qq = Set(Some(q));
    }
    if let Some(r) = remark {
        am.remark = Set(Some(r));
    }
    if let Some(rid) = role_id {
        // 角色变更仅 Admin 可操作；普通用户改自己时忽略传入的 role_id（防自提权限）
        if is_admin {
            am.role_id = Set(rid);
        }
    }

    sys_user_model::Entity::update_safety(am)?.exec(db).await?;
    Ok(CommonResponse::new(Ok(())))
}

pub async fn do_update_password(
    auth: AuthInfo,
    user_id: i64,
    old_password: String,
    new_password: String,
) -> Result<CommonResponse<()>> {
    let db = &DB_CONN.wait().pg_conn;
    // 权限校验：仅本人可凭旧密码改密；Admin 例外可改任意用户
    let is_admin = auth.info.role_id == SystemUserRole::Admin;
    if !is_admin && auth.info.id != user_id {
        return Err(anyhow!(
            "Forbidden: only the account owner can update password"
        ));
    }
    let m = sys_user_model::Entity::find_safety_by_id(user_id)
        .one(db)
        .await?;
    let m = m.ok_or(anyhow!("User not found"))?;

    // 校验旧密码，拒绝错误凭据
    if !_utils::bcrypt::verify_password(old_password, m.password.clone())? {
        return Err(anyhow!("Invalid old password"));
    }

    let mut am: sys_user_model::ActiveModel = m.into();
    am.password = Set(_utils::bcrypt::generate_storage_password(&new_password)?);
    sys_user_model::Entity::update_safety(am)?.exec(db).await?;
    Ok(CommonResponse::new(Ok(())))
}

pub async fn do_update_password_by_admin(
    _auth: AuthInfo,
    password: String,
    user_id: i64,
) -> Result<CommonResponse<()>> {
    let db = &DB_CONN.wait().pg_conn;
    let m = sys_user_model::Entity::find_safety_by_id(user_id)
        .one(db)
        .await?;
    let m = m.ok_or(anyhow!("User not found"))?;
    let mut am: sys_user_model::ActiveModel = m.into();
    am.password = Set(_utils::bcrypt::generate_storage_password(password)?);
    sys_user_model::Entity::update_safety(am)?.exec(db).await?;
    Ok(CommonResponse::new(Ok(())))
}

pub async fn do_delete(_auth: AuthInfo, work_id: i64) -> Result<()> {
    // 管理员删除用户：使用软删除 by id
    sys_user_model::Entity::delete_safety_by_id(work_id)?
        .exec(&DB_CONN.wait().pg_conn)
        .await?;
    Ok(())
}

pub async fn do_list(
    _auth: AuthInfo,
    pagination: Pagination,
    nickname: Option<String>,
    role_ids: Option<Vec<SystemUserRole>>,
    sort: Option<Vec<UserSort>>,
    username: Option<String>,
) -> Result<CommonResponse<serde_json::Value>> {
    let db = &DB_CONN.wait().pg_conn;

    let mut query = sys_user_model::Entity::find_safety();
    if let Some(nickname) = nickname
        && !nickname.is_empty()
    {
        query = query.filter(sys_user_model::Column::Nickname.like(nickname));
    }
    if let Some(username) = username
        && !username.is_empty()
    {
        query = query.filter(sys_user_model::Column::Username.eq(username));
    }
    if let Some(rids) = role_ids {
        query = query.filter(sys_user_model::Column::RoleId.is_in(rids));
    }

    // 排序：显式枚举映射（用户列表的 wire 契约是 serde rename，业务层按
    // 枚举变体匹配——变体重命名会变成编译错误，而非静默忽略排序键）。
    if let Some(sorts) = sort {
        use sea_orm::QueryOrder;
        for s in sorts {
            let (column, desc) = match s {
                UserSort::CreateTime => (sys_user_model::Column::CreateTime, false),
                UserSort::CreateTimeReverse => (sys_user_model::Column::CreateTime, true),
                UserSort::Id => (sys_user_model::Column::Id, false),
                UserSort::IdReverse => (sys_user_model::Column::Id, true),
                UserSort::Nickname => (sys_user_model::Column::Nickname, false),
                UserSort::NicknameReverse => (sys_user_model::Column::Nickname, true),
            };
            query = if desc {
                query.order_by(column, sea_orm::Order::Desc)
            } else {
                query.order_by(column, sea_orm::Order::Asc)
            };
        }
    }

    let size = pagination.size.unwrap_or(10) as u64;
    let current = pagination.current.unwrap_or(1);
    let offset = (current.saturating_sub(1) as u64).saturating_mul(size);

    let total = query.clone().count(db).await?;
    let items = query.limit(size).offset(offset).all(db).await?;

    let vos: Vec<SysUserVO> = items.into_iter().map(Into::into).collect();
    Ok(CommonResponse::new(Ok(
        serde_json::json!({"total": total, "record": vos}),
    )))
}

pub async fn do_kick_out(_auth: AuthInfo, work_id: String) -> Result<()> {
    // 踢出用户：删除该用户在 Redis 中的全部会话令牌。
    // JWT 本身无状态，登出/踢出依赖 Redis 会话（jwt:access:{uid}:{jti}）。
    let user_id = work_id
        .parse::<i64>()
        .map_err(|_| anyhow!("Invalid user id"))?;

    let Some(redis_client) = &DB_CONN.wait().redis_conn else {
        // Redis 不可用时退化为无操作（与 oauth 的降级策略一致）
        return Ok(());
    };
    let Ok(mut redis_conn) = redis_client.get_multiplexed_async_connection().await else {
        // Redis 连接失败同样降级（与 oauth 的降级策略一致）
        return Ok(());
    };

    let access_prefix = format!("jwt:access:{user_id}:*");
    let refresh_prefix = format!("jwt:refresh:{user_id}:*");
    // 用 SCAN + pipeline 而非 KEYS：KEYS 会阻塞 Redis 主线程（大 key 空间下
    // 长时间阻塞），SCAN 按游标分批遍历；收集后一次 pipeline 删除。
    for prefix in [access_prefix, refresh_prefix] {
        let keys = scan_keys(&mut redis_conn, &prefix).await?;
        if keys.is_empty() {
            continue;
        }
        let mut pipe = redis::pipe();
        for k in &keys {
            pipe.cmd("DEL").arg(k);
        }
        pipe.query_async::<()>(&mut redis_conn).await?;
    }
    Ok(())
}

/// 用 SCAN 游标分批收集匹配 pattern 的 key（不阻塞 Redis 主线程）。
async fn scan_keys(
    redis_conn: &mut redis::aio::MultiplexedConnection,
    pattern: &str,
) -> Result<Vec<String>> {
    let mut keys = Vec::new();
    let mut cursor: u64 = 0;
    loop {
        let (next, batch): (u64, Vec<String>) = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(pattern)
            .arg("COUNT")
            .arg(500)
            .query_async(redis_conn)
            .await?;
        keys.extend(batch);
        cursor = next;
        if cursor == 0 {
            break;
        }
    }
    Ok(keys)
}
