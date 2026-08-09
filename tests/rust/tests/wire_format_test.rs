//! Wire-format compatibility tests against the **real database data shapes**
//! (see the audit report `db_audit.md`: ddns.minemc.top:13070/genshin_map).
//!
//! These are pure ser/de unit tests — no DB, no tokio. Every sample constant
//! is an inline copy of a shape observed in production rows, so the suite
//! fails loudly the day a VO/enum drifts from the Java-era contract.
//!
//! Coverage:
//! - sys_user.access_policy: prefixed form round-trips; the 2 unprefixed
//!   legacy rows (id=194/195) are documented as a P1 gap (ignored test).
//! - history.content: Java JSON snapshot strings pass through verbatim.
//! - marker.position: single "{x},{y}" string on the wire.
//! - notice.channel: JSON array of uppercase channel names.
//! - marker_linkage.link_action: uppercase enum strings ("TRIGGER_ALL").
//! - sys_user.password: `{bcrypt}`-prefixed storage (68 chars).

use _database::models::common::notice::ChannelWrapper;
use _utils::bcrypt;
use _utils::models::{
    history::HistoryItemVO,
    marker::{MarkerItemLinkVo, MarkerVO},
    marker_link::MarkerLinkVO,
    notice::{NoticeChannel, NoticeVO},
};
use _utils::types::{
    AccessPolicyItemEnum, AccessPolicyList, HiddenFlag, HistoryEditType, HistoryOperationType,
    MarkerLinkageLinkAction,
};

// ─────────────────────────────────────────────────────────────────────────────
// 1. sys_user.access_policy — prefixed form (197/199 rows) round-trips
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn access_policy_prefixed_roundtrip() {
    // The prefixed form is what 197 of 199 real rows store and what the
    // business layer writes; it must never regress.
    let wire = r#"["ip:same_last_ip","dev:same_last_device"]"#;
    let parsed: Vec<AccessPolicyItemEnum> =
        serde_json::from_str(wire).expect("prefixed form deserializes");
    assert_eq!(
        parsed,
        vec![
            AccessPolicyItemEnum::IpSameLastIp,
            AccessPolicyItemEnum::DevSameLastDevice
        ]
    );
    // Serialization keeps the prefix (storage/wire contract).
    assert_eq!(
        serde_json::to_string(&parsed).expect("serialize"),
        wire,
        "prefixes must survive a round-trip"
    );
    // DB json-column path goes through the AccessPolicyList wrapper.
    let wrapped =
        serde_json::from_value::<AccessPolicyList>(serde_json::json!(["ip:same_last_ip"]))
            .expect("AccessPolicyList from stored json");
    assert_eq!(wrapped.0, vec![AccessPolicyItemEnum::IpSameLastIp]);
}

#[test]
fn access_policy_unprefixed_legacy_rows_deserialize() {
    // Real data shape (audit, sys_user id=194 kafka / id=195 firefly, both
    // del_flag=false, role_id=0 Admin): the two unprefixed strings below are
    // the exact values stored in the access_policy json column. Any query
    // hitting these rows currently fails to deserialize the whole row.
    let real_rows = r#"["same_last_ip","same_last_device"]"#;
    let parsed: Vec<AccessPolicyItemEnum> = serde_json::from_str(real_rows)
        .unwrap_or_else(|e| panic!("unprefixed legacy rows must deserialize (P1): {e}"));
    assert_eq!(
        parsed,
        vec![
            AccessPolicyItemEnum::IpSameLastIp,
            AccessPolicyItemEnum::DevSameLastDevice
        ]
    );
    // The DB wrapper must also survive the same input.
    let wrapped = serde_json::from_value::<AccessPolicyList>(serde_json::json!([
        "same_last_ip",
        "same_last_device"
    ]))
    .expect("AccessPolicyList from unprefixed legacy json");
    assert_eq!(wrapped.0.len(), 2);
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. history.content — Java JSON snapshot string passes through verbatim
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn history_content_snapshot_passthrough() {
    // Real DB shape (audit, type=4 打点 rows): content is a Java-contract JSON
    // *snapshot string* with top-level content/hiddenFlag/id/itemList/
    // markerCreatorId/markerTitle/picture/pictureCreatorId/position/
    // refreshTime/videoPath. The backend VO must keep it an opaque string.
    const SNAPSHOT: &str = r#"{"content":"风滚草","hiddenFlag":0,"id":66181,"itemList":[{"count":1,"itemId":2992}],"markerCreatorId":28,"markerTitle":"风滚草","picture":"","pictureCreatorId":0,"position":"-7507.75,2244.25","refreshTime":43200000,"videoPath":""}"#;

    let vo = HistoryItemVO {
        version: 0,
        id: 58502,
        create_time: 0.0,
        update_time: None,
        creator_id: Some(46),
        updater_id: None,
        del_flag: false,
        md5: Some("0123456789abcdef0123456789abcdef".into()),
        ipv4: None,
        t_id: 49232,
        history_type: Some(HistoryOperationType::Position),
        edit_type: HistoryEditType::Modified,
        content: SNAPSHOT.to_string(),
    };

    let json = serde_json::to_value(&vo).expect("serialize HistoryItemVO");
    // content is a string on the wire — never a re-parsed nested object.
    assert!(
        json["content"].is_string(),
        "content must stay an opaque string, got: {}",
        json["content"]
    );
    assert_eq!(
        json["content"], SNAPSHOT,
        "content must pass through byte-verbatim"
    );
    // Java wire keys for the rest of the VO.
    assert_eq!(json["tid"], 49232, "t_id serializes as `tid`");
    assert_eq!(json["type"], 4, "historyType serializes as numeric `type`");
    assert_eq!(json["editType"], 2);

    let back: HistoryItemVO = serde_json::from_value(json).expect("deserialize HistoryItemVO");
    assert_eq!(back.content, SNAPSHOT);
    assert_eq!(back.t_id, 49232);
    assert_eq!(back.history_type, Some(HistoryOperationType::Position));
    assert_eq!(back.edit_type, HistoryEditType::Modified);
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. marker.position — single "{x},{y}" string on the wire
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn marker_position_verbatim_string() {
    // Real DB shape (audit): position is one text column, e.g. '-7507.75,2244.25'.
    let vo = MarkerVO {
        version: 0,
        id: 66181,
        create_time: 0.0,
        update_time: None,
        creator_id: None,
        updater_id: None,
        del_flag: false,
        marker_stamp: None,
        marker_title: Some("风滚草".into()),
        position: "-7507.75,2244.25".into(),
        content: Some("风滚草".into()),
        picture: Some(String::new()),
        marker_creator_id: 28,
        picture_creator_id: Some(0),
        video_path: Some(String::new()),
        refresh_time: 43200000,
        hidden_flag: HiddenFlag::Visible,
        extra: Some(serde_json::json!({})),
        item_list: vec![MarkerItemLinkVo {
            item_id: 2992,
            count: 1,
            icon_tag: None,
            icon_id: 0,
        }],
        linkage_id: None,
    };

    let json = serde_json::to_value(&vo).expect("serialize MarkerVO");
    assert!(
        json["position"].is_string(),
        "position must stay a single string, got: {}",
        json["position"]
    );
    assert_eq!(
        json["position"], "-7507.75,2244.25",
        "position must not be split into x/y numbers"
    );
    // itemList survives as the Java camelCase array.
    assert_eq!(json["itemList"][0]["itemId"], 2992);
    assert_eq!(json["itemList"][0]["count"], 1);

    let back: MarkerVO = serde_json::from_value(json).expect("deserialize MarkerVO");
    assert_eq!(back.position, "-7507.75,2244.25");
    assert_eq!(back.hidden_flag, HiddenFlag::Visible);
    assert_eq!(back.item_list[0].item_id, 2992);
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. notice.channel — JSON array of uppercase channel names
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn notice_channel_array_wire() {
    // Real DB shape (audit): channel is a json array of strings, e.g.
    // `['DASHBOARD']` or `['COMMON']`. The VO must serialize the array under
    // the `channel` key and deserialize the stored array back.
    let vo = NoticeVO {
        version: 0,
        id: 6,
        create_time: 0.0,
        update_time: None,
        creator_id: None,
        updater_id: None,
        title: "待生效公告".into(),
        content: Some("<p>你好世界</p>".into()),
        channels: vec![NoticeChannel::Dashboard, NoticeChannel::Common],
        sort_index: 10,
        valid_time_start: None,
        valid_time_end: None,
    };

    let json = serde_json::to_value(&vo).expect("serialize NoticeVO");
    assert!(
        json["channel"].is_array(),
        "channel must be an array on the wire"
    );
    assert_eq!(json["channel"], serde_json::json!(["DASHBOARD", "COMMON"]));

    // DB json-column wrapper (ChannelWrapper) round-trips the stored array.
    let wrapped = serde_json::from_value::<ChannelWrapper>(serde_json::json!(["DASHBOARD"]))
        .expect("ChannelWrapper from stored json");
    assert_eq!(wrapped.0, vec!["DASHBOARD".to_string()]);
    assert_eq!(
        serde_json::to_value(&wrapped).expect("serialize ChannelWrapper"),
        serde_json::json!(["DASHBOARD"])
    );

    // Round-trip the wire shape back into the VO.
    let back: NoticeVO = serde_json::from_value(json).expect("deserialize NoticeVO");
    assert_eq!(
        back.channels,
        vec![NoticeChannel::Dashboard, NoticeChannel::Common]
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. marker_linkage.link_action — uppercase enum strings
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn marker_linkage_link_action_uppercase() {
    // Real DB shape (audit, 3,152 rows): link_action is an uppercase string,
    // e.g. 'TRIGGER_ALL'; all values are in the enum range.
    for (wire, expected) in [
        ("TRIGGER", MarkerLinkageLinkAction::Trigger),
        ("TRIGGER_ALL", MarkerLinkageLinkAction::TriggerAll),
        ("TRIGGER_ANY", MarkerLinkageLinkAction::TriggerAny),
        ("RELATED", MarkerLinkageLinkAction::Related),
        ("DIRECTED", MarkerLinkageLinkAction::Directed),
        ("PATH_UNI_DIR", MarkerLinkageLinkAction::PathUniDir),
        ("PATH_BI_DIR", MarkerLinkageLinkAction::PathBiDir),
        ("EQUIVALENT", MarkerLinkageLinkAction::Equivalent),
    ] {
        let parsed: MarkerLinkageLinkAction =
            serde_json::from_str(&format!("\"{wire}\"")).expect("link_action deserializes");
        assert_eq!(parsed, expected);
        assert_eq!(
            serde_json::to_string(&parsed).expect("serialize"),
            format!("\"{wire}\""),
            "link_action round-trip must preserve the uppercase form"
        );
    }

    // Wire VO (camelCase linkAction) carrying the audited row shape.
    let link = serde_json::json!({
        "version": 0,
        "id": 7,
        "creatorId": null,
        "updaterId": null,
        "updateTime": null,
        "groupId": "851168d7d77d434e93881e504f2a4df1",
        "fromId": 71243,
        "toId": 71244,
        "linkAction": "TRIGGER_ALL",
        "linkReverse": true,
        "path": []
    });
    let vo: MarkerLinkVO = serde_json::from_value(link).expect("MarkerLinkVO deserializes");
    assert_eq!(vo.link_action, Some(MarkerLinkageLinkAction::TriggerAll));
    assert_eq!(vo.from_id, 71243);
    assert_eq!(vo.to_id, 71244);
    assert_eq!(
        vo.group_id.as_deref(),
        Some("851168d7d77d434e93881e504f2a4df1")
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. sys_user.password — `{bcrypt}`-prefixed storage (68 chars)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sys_user_password_prefix() {
    // Real DB shape (audit): password = `{bcrypt}$2a$...` (68 chars total).
    let stored = bcrypt::generate_storage_password("pw123").expect("generate storage password");
    assert!(
        stored.starts_with("{bcrypt}"),
        "storage uses the {{bcrypt}} prefix"
    );
    assert_eq!(
        stored.len(),
        68,
        "{{bcrypt}}(8) + bcrypt hash(60) = 68 chars, got: {}",
        stored.len()
    );

    // verify_password understands the prefixed form (login path).
    assert!(_utils::bcrypt::verify_password("pw123", &stored).expect("verify ok"));
    assert!(!_utils::bcrypt::verify_password("wrong", &stored).expect("verify ok"));

    // A raw hash without the prefix also verifies (legacy rows).
    let raw = bcrypt::generate_hash("pw123").expect("generate hash");
    assert_eq!(raw.len(), 60, "bcrypt hash alone is 60 chars");
    assert!(bcrypt::verify_password("pw123", &raw).expect("verify raw hash"));

    // A synthetic row shaped exactly like the audited ones.
    let synthetic = format!("{{bcrypt}}{raw}");
    assert_eq!(synthetic.len(), 68);
    assert!(bcrypt::verify_password("pw123", &synthetic).expect("verify synthetic row"));
    assert!(!bcrypt::verify_password("nope", &synthetic).expect("verify synthetic row"));
}
