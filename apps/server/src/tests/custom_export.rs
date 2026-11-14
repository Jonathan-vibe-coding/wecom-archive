use super::*;
use archive_export::{ExportCustomization, ExportTimezone};

fn fixture() -> ClientExportV1 {
    let mut export: ClientExportV1 = serde_json::from_slice(&export_bytes(
        "selected body",
        Uuid::new_v4(),
        "synthetic-custom-export",
    ))
    .unwrap();
    let template = export.batches[0].messages[0].clone();
    export.batches[0].messages = (0..6)
        .map(|index| {
            let mut message = template.clone();
            message.stable_message_id = format!("synthetic-{index}");
            message.source_message_id = format!("synthetic-source-{index}");
            message.sent_at = Utc.with_ymd_and_hms(2026, 1, 1, 16, 0, index).unwrap();
            if index > 0 {
                message.body_text = Some(format!("excluded body {index}"));
            }
            match index {
                1 => {
                    message.message_type = MessageType::System;
                    message.direction = MessageDirection::System;
                }
                2 => message.lifecycle = LifecycleState::Recalled,
                3 => message.lifecycle = LifecycleState::DeletedAtSource,
                4 => {
                    message.message_type = MessageType::System;
                    message.raw_type = "group_announcement".into();
                }
                5 => {
                    message.message_type = MessageType::System;
                    message.raw_type = "2063".into();
                }
                _ => {}
            }
            if index > 0 {
                message.media.push(MediaRefV1 {
                    content_hash: None,
                    original_name: Some("excluded.txt".into()),
                    mime_type: None,
                    size_bytes: None,
                    source_locator: "synthetic".into(),
                    integrity: MediaIntegrity::Missing,
                    missing_reason: Some("synthetic".into()),
                });
            }
            message
        })
        .collect();
    archive_transfer::create_export(
        &export.client_version,
        export.batches,
        export.conversations,
        export.participants,
    )
    .unwrap()
}

fn options() -> ExportCustomization {
    serde_json::from_value(json!({
        "includeSystemMessages": false, "includeRecalledMessages": false,
        "includeDeletedMessages": false, "includeAnnouncements": false,
        "layout": "compact", "timezone": "asia_shanghai",
        "fields": {"conversation": false, "messageType": false, "lifecycle": false, "direction": false}
    })).unwrap()
}

#[test]
fn dates_use_the_selected_timezone_and_keep_exclusive_end() {
    let start = parse_export_date(Some("2026-01-02"), false, ExportTimezone::AsiaShanghai)
        .unwrap()
        .unwrap();
    let end = parse_export_date(Some("2026-01-02"), true, ExportTimezone::AsiaShanghai)
        .unwrap()
        .unwrap();
    assert_eq!(start.to_rfc3339(), "2026-01-01T16:00:00+00:00");
    assert_eq!(end - start, TimeDelta::days(1));
    assert_eq!(
        parse_export_date(Some("2026-01-02"), false, ExportTimezone::Utc)
            .unwrap()
            .unwrap()
            - start,
        TimeDelta::hours(8)
    );
    assert!(parse_export_date(Some("invalid"), false, ExportTimezone::Utc).is_err());
}

#[test]
fn local_filtered_json_has_valid_counts_and_checksum_and_accepts_empty_selection() {
    let directory = TestDirectory::new();
    let mut export = fixture();
    let id = export.export_id;
    filter_local_export(&mut export, Some(&options())).unwrap();
    assert_eq!(
        (
            export.message_count,
            export.media_count,
            export.missing_media_count
        ),
        (1, 0, 0)
    );
    assert_eq!(export.export_id, id);
    let target = directory.0.join("custom.json");
    archive_transfer::write_importable_json_formatted(&export, &target, true).unwrap();
    let decoded = read_json(&target).unwrap();
    assert_eq!(
        decoded.batches[0].messages[0].body_text.as_deref(),
        Some("selected body")
    );
    assert_eq!(
        decoded.batches[0].messages[0].sender_id.as_deref(),
        Some("participant-fixture")
    );
    let empty = ExportCustomization {
        message_types: Some(Vec::new()),
        ..options()
    };
    filter_local_export(&mut export, Some(&empty)).unwrap();
    export.validate().unwrap();
    assert_eq!(export.message_count, 0);
    assert!(
        export.batches.is_empty()
            && export.conversations.is_empty()
            && export.media_blobs.is_empty()
    );
}

#[tokio::test]
async fn archive_filters_apply_to_markdown_and_importable_json_with_timezone_boundaries() {
    let directory = TestDirectory::new();
    let app_state = state(&directory);
    enable_super_admin(&app_state);
    ingest_export(&app_state, fixture()).await.unwrap();
    for format in ["md", "json"] {
        let request = serde_json::from_value(json!({
            "scope": "date_range", "format": format,
            "startDate": "2026-01-02", "endDate": "2026-01-02",
            "pretty": false, "customization": options()
        }))
        .unwrap();
        let result = create_export_job(
            State(app_state.clone()),
            authorized_headers(),
            Json(request),
        )
        .await
        .unwrap();
        assert_eq!(
            (
                result.message_count,
                result.media_count,
                result.missing_media_count
            ),
            (1, 0, 0)
        );
        let target = directory.0.join("exports").join(result.file_name);
        if format == "json" {
            let export = read_json(&target).unwrap();
            assert_eq!(export.message_count, 1);
            assert_eq!(
                export.batches[0].messages[0].lifecycle,
                LifecycleState::Active
            );
        } else {
            let body = std::fs::read_to_string(target).unwrap();
            assert!(body.contains("selected body") && body.contains("2026-01-02 00:00:00+08:00"));
            assert!(body.contains("测试成员"));
            assert!(!body.contains("excluded body") && !body.contains("excluded.txt"));
        }
    }
}
