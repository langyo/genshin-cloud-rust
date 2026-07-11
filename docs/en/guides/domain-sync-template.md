# Domain Sync Template

A reusable checklist for porting a single domain from the Java backend to
Rust. The area domain is used as the concrete example — follow this pattern
for every new domain (icon, item, tag, notice, ...).

## The five layers

Every domain touches exactly five files, one per layer. Port them in order:

### 1. sea-orm entity — `packages/database/src/models/<domain>/<domain>.rs`

Define the `Model`, `ActiveModel` (via `DeriveEntityModel`), `Column`,
`Relation`, and the `impl_safe_operation!` invocation. The entity **must**
include `version` (optimistic lock), `del_flag` (soft delete), and — for
content entities — `hidden_flag` (data-level filtering).

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Deserialize, Serialize)]
#[sea_orm(table_name = "area", schema_name = "genshin_map")]
pub struct Model {
    pub version: i64,
    #[sea_orm(primary_key)]
    pub id: i64,
    // ... common fields (create_time, update_time, creator_id, del_flag) ...
    pub hidden_flag: HiddenFlag,
    pub special_flag: i32,
    // ... domain-specific fields ...
}

impl_safe_operation! {
    active_model_ty: ActiveModel,
    updated_at_column_name: update_time,
    updated_at_column_init_expr: chrono::Utc::now().naive_utc(),
    del_flag_column: Column::DelFlag
}
```

### 2. DTO / VO types — `packages/utils/src/models/<domain>.rs`

Request and response shapes, serialized as `camelCase` to match the Java API
contract. Add the module to `packages/utils/src/models/mod.rs`.

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AreaListRequest {
    pub parent_id: Option<i64>,
    pub hidden_flag: Option<crate::types::HiddenFlag>,
}
```

### 3. Business logic — `packages/functions/src/functions/api/<domain>.rs`

Pure async functions (`do_list`, `do_get`, `do_add`, `do_update`,
`do_delete`) that orchestrate entity reads/writes. No HTTP types here — they
take DTOs and return `CommonResponse<T>`.

```rust
pub async fn do_list(_auth: AuthInfo, payload: AreaListRequest)
    -> Result<CommonResponse<AreaListResponse>>
{
    let mut query = area_model::Entity::find_safety();
    if let Some(hf) = payload.hidden_flag {
        query = query.filter(area_model::Column::HiddenFlag.eq(hf));
    }
    // ...
}
```

### 4. axum routes — `packages/router/src/routes/api/<domain>/`

One file per verb (`add.rs`, `get.rs`, `list.rs`, `update.rs`, `delete.rs`,
`mod.rs`). Extract `ExtractAuthInfo` + the request body, delegate to the
`do_*` function. Register the router in the parent `mod.rs`.

### 5. Smoke tests — `tests/rust/tests/<domain>/`

At minimum, assert the entity table name matches Java and that the
`hidden_flag` / `version` / `del_flag` columns exist (see
`tests/rust/tests/area/area_domain_test.rs`). Add DB-backed integration tests
under `#[ignore]` once the docker-compose harness is wired.

## Checklist

- [ ] Entity matches Java table name + schema (`genshin_map`)
- [ ] `version`, `del_flag`, `hidden_flag` columns present
- [ ] `impl_safe_operation!` invoked (gives `find_safety` / `update_safety` /
      `delete_safety`)
- [ ] DTO/VO fields are `camelCase` (Java parity)
- [ ] `do_list` supports `hidden_flag` filtering where relevant
- [ ] Route handlers registered in the parent module
- [ ] Smoke test asserts table name + key columns
- [ ] CHANGELOG entry added
