# Java Sync Roadmap

This roadmap tracks the work to bring the Rust backend to feature parity with
the Java reference implementation
([`java-genshin-map-cloud`](https://github.com/kongying-tavern/java-genshin-map-cloud)).

## Java-side scope

The Java backend (~30 controllers, ~20 entities) covers:

- **Map content**: area, icon, icon_type, item, item_type, marker,
  marker_link, tag, tag_type, route, notice, history, score.
- **Binary archive export** (`*_doc` endpoints): large datasets are serialized,
  compressed, and keyed by MD5 so clients can incrementally sync.
- **Crowd-sourced punctuate workflow**: user marker submissions → staging
  table → audit → promotion to live markers.
- **System**: user, role, device (login anomaly detection), invitation,
  action_log, archive.
- **Auth**: OAuth2 password-grant JWT with JWKS endpoint, RSA keypair,
  device/IP anomaly detection on token issuance.

## Porting priority

| # | Domain / feature | Key entity / concept | Complexity |
|---|---|---|---|
| 1 | **area + marker** (reference samples) | `Area`, `Marker`, `hiddenFlag`, `special_flag` | Done — used as the porting template |
| 2 | icon / icon_type | `Icon`, `IconType`, icon-tag merge | Low |
| 3 | item / item_type | `Item`, `ItemType`, `selectPageItemByCondition` (the `specialFlag` filter) | Medium |
| 4 | tag / tag_type | `Tag`, `TagType` | Low |
| 5 | notice / route / history | `Notice` (validity-sort rule), `Route`, `History` | Low–Medium |
| 6 | punctuate workflow | `MarkerPunctuate` staging → `Marker` promotion, 3-state audit | High |
| 7 | scoring | `ScoreStat`, scope/span bucketing, aggregation | High |
| 8 | system (user/role/device/invitation/action_log) | `SysUser*`, login anomaly detection | Medium |
| 9 | BinaryMD5 archive export (`*_doc`) | compressed MD5-keyed cache, two-tier Caffeine → port to `moka`/`quick-cache` | High |
| 10 | OAuth2 / JWKS | RSA keypair, JWT token enhancer, device/IP check | High |

## Notes

- Items 2–5 are low-risk CRUD ports following the
  [domain-sync template](./domain-sync-template.md).
- Items 6–10 each carry significant business logic and should get their own
  design doc under `docs/en/designs/` before implementation.
- The Rust side already shares the same PostgreSQL schema as the Java side
  (verified by table-name parity tests), so the two backends can run against
  the same database during the migration.
