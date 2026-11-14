//! Reading-format preferences. These never mutate the archived MessageV1.
use super::*;
use archive_domain::{LifecycleState, MessageType};
use chrono::FixedOffset;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExportLayout {
    #[default]
    Standard,
    Compact,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExportTimezone {
    #[default]
    Utc,
    AsiaShanghai,
}

impl ExportTimezone {
    pub fn offset(&self) -> FixedOffset {
        FixedOffset::east_opt(match self {
            Self::Utc => 0,
            Self::AsiaShanghai => 8 * 60 * 60,
        })
        .expect("export timezones are fixed valid offsets")
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Utc => "UTC",
            Self::AsiaShanghai => "北京时间（UTC+08:00）",
        }
    }

    fn format(&self, value: &DateTime<Utc>) -> String {
        value
            .with_timezone(&self.offset())
            .format("%Y-%m-%d %H:%M:%S%:z")
            .to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportFields {
    pub conversation: bool,
    pub sender: bool,
    pub timestamp: bool,
    pub message_type: bool,
    pub lifecycle: bool,
    pub direction: bool,
    pub body: bool,
    pub media: bool,
    pub quote: bool,
    pub technical_ids: bool,
}

impl Default for ExportFields {
    fn default() -> Self {
        Self {
            conversation: true,
            sender: true,
            timestamp: true,
            message_type: true,
            lifecycle: true,
            direction: true,
            body: true,
            media: true,
            quote: true,
            technical_ids: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportCustomization {
    pub fields: ExportFields,
    /// Other system messages; announcements and recalls have separate switches.
    pub include_system_messages: bool,
    pub include_recalled_messages: bool,
    pub include_deleted_messages: bool,
    pub include_announcements: bool,
    /// None means all types; an explicitly empty selection means no messages.
    pub message_types: Option<Vec<MessageType>>,
    pub layout: ExportLayout,
    pub timezone: ExportTimezone,
}

impl Default for ExportCustomization {
    fn default() -> Self {
        Self {
            fields: ExportFields::default(),
            include_system_messages: true,
            include_recalled_messages: true,
            include_deleted_messages: true,
            include_announcements: true,
            message_types: None,
            layout: ExportLayout::Standard,
            timezone: ExportTimezone::Utc,
        }
    }
}

/// Shared predicate for the in-memory exporter and archive-source count/visit
/// passes. Explicit customization replaces simplify's implicit announcement
/// filter. Type selection is an additional intersection, never a union.
pub fn includes_message(
    message: &MessageV1,
    simplify: bool,
    customization: Option<&ExportCustomization>,
) -> bool {
    let announcement = matches!(
        message.raw_type.as_str(),
        "group_announcement" | "group_board"
    );
    let Some(options) = customization else {
        return !simplify || !announcement;
    };
    if options
        .message_types
        .as_ref()
        .is_some_and(|types| !types.contains(&message.message_type))
    {
        return false;
    }
    let recalled =
        message.lifecycle == LifecycleState::Recalled || message.raw_type.trim() == "2063";
    if (!options.include_announcements && announcement)
        || (!options.include_recalled_messages && recalled)
        || (!options.include_deleted_messages
            && message.lifecycle == LifecycleState::DeletedAtSource)
    {
        return false;
    }
    let other_system = !announcement
        && !recalled
        && (message.message_type == MessageType::System
            || message.direction == MessageDirection::System);
    options.include_system_messages || !other_system
}

#[derive(Clone, Copy)]
enum Field {
    Conversation,
    Time,
    Sender,
    Type,
    Lifecycle,
    Direction,
    Body,
    Media,
    Quote,
    MessageId,
    ConversationId,
    SenderId,
    RawType,
    MediaHashes,
}

impl Field {
    fn label(self, pretty: bool) -> &'static str {
        match (self, pretty) {
            (Self::Conversation, true) => "会话",
            (Self::Conversation, false) => "conversation",
            (Self::Time, true) => "时间",
            (Self::Time, false) => "sent_at",
            (Self::Sender, true) => "发送人",
            (Self::Sender, false) => "sender",
            (Self::Type, true) => "消息类型",
            (Self::Type, false) => "message_type",
            (Self::Lifecycle, true) => "状态",
            (Self::Lifecycle, false) => "lifecycle",
            (Self::Direction, true) => "方向",
            (Self::Direction, false) => "direction",
            (Self::Body, true) => "正文",
            (Self::Body, false) => "body_text",
            (Self::Media, true) => "附件",
            (Self::Media, false) => "media_names",
            (Self::Quote, true) => "引用消息",
            (Self::Quote, false) => "quoted_message_id",
            (Self::MessageId, true) => "消息 ID",
            (Self::MessageId, false) => "stable_message_id",
            (Self::ConversationId, true) => "会话 ID",
            (Self::ConversationId, false) => "conversation_id",
            (Self::SenderId, true) => "发送人 ID",
            (Self::SenderId, false) => "sender_id",
            (Self::RawType, true) => "原始类型",
            (Self::RawType, false) => "raw_type",
            (Self::MediaHashes, true) => "附件哈希",
            (Self::MediaHashes, false) => "media_hashes",
        }
    }

    fn value(
        self,
        package: &ExportPackage,
        names: &ExportNames<'_>,
        message: &MessageV1,
        options: &ExportCustomization,
    ) -> Result<String, ExportError> {
        Ok(match self {
            Self::Conversation => names.conversation(message).into(),
            Self::Time => options.timezone.format(&message.sent_at),
            Self::Sender => names.sender(message).into(),
            Self::Type => display_message_type(message, package.pretty)?,
            Self::Lifecycle => enum_text(&message.lifecycle)?,
            Self::Direction => enum_text(&message.direction)?,
            Self::Body => message.body_text.clone().unwrap_or_default(),
            Self::Media => message
                .media
                .iter()
                .map(|item| item.original_name.as_deref().unwrap_or("未命名附件"))
                .collect::<Vec<_>>()
                .join(" | "),
            Self::Quote => message.quoted_message_id.clone().unwrap_or_default(),
            Self::MessageId => message.stable_message_id.clone(),
            Self::ConversationId => message.conversation_id.clone(),
            Self::SenderId => message.sender_id.clone().unwrap_or_default(),
            Self::RawType => message.raw_type.clone(),
            Self::MediaHashes => message
                .media
                .iter()
                .filter_map(|item| item.content_hash.as_deref())
                .collect::<Vec<_>>()
                .join(" | "),
        })
    }
}

fn columns(options: &ExportCustomization) -> Vec<Field> {
    let f = &options.fields;
    let mut fields = Vec::new();
    for (enabled, field) in [
        (f.conversation, Field::Conversation),
        (f.timestamp, Field::Time),
        (f.sender, Field::Sender),
        (f.message_type, Field::Type),
        (f.lifecycle, Field::Lifecycle),
        (f.direction, Field::Direction),
        (f.body, Field::Body),
        (f.media, Field::Media),
        (f.quote, Field::Quote),
    ] {
        if enabled {
            fields.push(field);
        }
    }
    if f.technical_ids {
        fields.extend([
            Field::MessageId,
            Field::ConversationId,
            Field::SenderId,
            Field::RawType,
        ]);
        if f.media {
            fields.push(Field::MediaHashes);
        }
    }
    fields
}

pub(super) fn csv_header(
    output: &mut csv::Writer<File>,
    package: &ExportPackage,
    options: &ExportCustomization,
) -> Result<(), ExportError> {
    output.write_record(
        columns(options)
            .into_iter()
            .map(|field| field.label(package.pretty)),
    )?;
    Ok(())
}

pub(super) fn csv_message(
    output: &mut csv::Writer<File>,
    package: &ExportPackage,
    names: &ExportNames<'_>,
    message: &MessageV1,
    options: &ExportCustomization,
) -> Result<(), ExportError> {
    let cells = columns(options)
        .into_iter()
        .map(|field| {
            field
                .value(package, names, message, options)
                .map(|value| safe_csv(&value))
        })
        .collect::<Result<Vec<_>, _>>()?;
    output.write_record(cells)?;
    Ok(())
}

pub(super) fn html_header(
    output: &mut impl Write,
    package: &ExportPackage,
    options: &ExportCustomization,
    for_print: bool,
    count: u64,
) -> Result<(), ExportError> {
    output.write_all(
        html_document_start(
            "归档导出",
            for_print,
            options.layout == ExportLayout::Standard && package.pretty,
        )
        .as_bytes(),
    )?;
    write!(output, "<section class=\"summary\"><p>{count} 条消息")?;
    if options.fields.timestamp {
        write!(output, " · {}", options.timezone.label())?;
    }
    output.write_all(b"</p></section>")?;
    Ok(())
}

pub(super) fn html_conversation(
    output: &mut impl Write,
    names: &ExportNames<'_>,
    id: &str,
    options: &ExportCustomization,
) -> Result<(), ExportError> {
    output.write_all(b"<section class=\"conversation\">")?;
    if options.fields.conversation {
        write!(
            output,
            "<h2>{}</h2>",
            html_escape(names.conversation_id(id))
        )?;
    }
    Ok(())
}

pub(super) fn html_message(
    output: &mut impl Write,
    package: &ExportPackage,
    names: &ExportNames<'_>,
    message: &MessageV1,
    options: &ExportCustomization,
) -> Result<(), ExportError> {
    output.write_all(b"<article class=\"message\">")?;
    let mut header = Vec::new();
    for field in columns(options) {
        if matches!(field, Field::Conversation | Field::Body | Field::Media) {
            continue;
        }
        let value = field.value(package, names, message, options)?;
        if value.is_empty() {
            continue;
        }
        let display = if options.layout == ExportLayout::Compact
            && matches!(field, Field::Time | Field::Sender)
        {
            value
        } else {
            format!("{}：{value}", field.label(true))
        };
        header.push(format!("<span>{}</span>", html_escape(&display)));
    }
    if !header.is_empty() {
        write!(output, "<header>{}</header>", header.join(" "))?;
    }
    if options.fields.body {
        write!(
            output,
            "<p>{}</p>",
            html_escape(message.body_text.as_deref().unwrap_or_default())
        )?;
    }
    if options.fields.media && !message.media.is_empty() {
        write!(
            output,
            "<small class=\"media\">附件：{}</small>",
            html_escape(&Field::Media.value(package, names, message, options)?)
        )?;
    }
    output.write_all(b"</article>")?;
    Ok(())
}

pub(super) fn markdown_header(
    output: &mut impl Write,
    options: &ExportCustomization,
    count: u64,
) -> Result<(), ExportError> {
    writeln!(
        output,
        "# 归档导出\n\n{count} 条消息{}\n",
        if options.fields.timestamp {
            format!(" · {}", options.timezone.label())
        } else {
            String::new()
        }
    )?;
    Ok(())
}

pub(super) fn markdown_conversation(
    output: &mut impl Write,
    names: &ExportNames<'_>,
    id: &str,
    options: &ExportCustomization,
) -> Result<(), ExportError> {
    if options.fields.conversation {
        writeln!(
            output,
            "## {}\n",
            markdown_escape(names.conversation_id(id))
        )?;
    }
    Ok(())
}

pub(super) fn markdown_message(
    output: &mut impl Write,
    package: &ExportPackage,
    names: &ExportNames<'_>,
    message: &MessageV1,
    options: &ExportCustomization,
) -> Result<(), ExportError> {
    let mut metadata = Vec::new();
    for field in columns(options) {
        if matches!(field, Field::Conversation | Field::Body | Field::Media) {
            continue;
        }
        let value = field.value(package, names, message, options)?;
        if value.is_empty() {
            continue;
        }
        let value = if options.layout == ExportLayout::Compact
            && matches!(field, Field::Time | Field::Sender)
        {
            value
        } else {
            format!("{}：{value}", field.label(true))
        };
        metadata.push(markdown_escape(&value));
    }
    if !metadata.is_empty() {
        if options.layout == ExportLayout::Compact {
            writeln!(output, "**{}**\n", metadata.join(" · "))?;
        } else {
            writeln!(output, "### {}\n", metadata.join(" · "))?;
        }
    }
    if options.fields.body
        && let Some(body) = &message.body_text
    {
        // Fencing preserves text (including code and Markdown) exactly and
        // prevents source messages becoming executable/remote markup.
        let longest = body.split(|c| c != '`').map(str::len).max().unwrap_or(0);
        let fence = "`".repeat(3.max(longest + 1));
        writeln!(output, "{fence}text\n{body}\n{fence}\n")?;
    }
    if options.fields.media && !message.media.is_empty() {
        writeln!(
            output,
            "附件：{}\n",
            markdown_escape(&Field::Media.value(package, names, message, options)?)
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compact_options() -> ExportCustomization {
        ExportCustomization {
            fields: ExportFields {
                conversation: false,
                sender: true,
                timestamp: true,
                message_type: false,
                lifecycle: false,
                direction: false,
                body: true,
                media: false,
                quote: false,
                technical_ids: false,
            },
            include_announcements: false,
            layout: ExportLayout::Compact,
            timezone: ExportTimezone::AsiaShanghai,
            ..ExportCustomization::default()
        }
    }

    #[test]
    fn independent_filters_intersect_with_type_selection() {
        let mut message = crate::tests::fixture(ExportFormat::Md, false, false)
            .messages
            .remove(0);
        let mut options = ExportCustomization {
            include_system_messages: false,
            ..ExportCustomization::default()
        };
        assert!(includes_message(&message, true, Some(&options)));
        message.direction = MessageDirection::System;
        assert!(!includes_message(&message, false, Some(&options)));
        message.direction = MessageDirection::Incoming;
        message.message_type = MessageType::System;
        assert!(!includes_message(&message, false, Some(&options)));

        // Announcements and recalls are independent of other system messages.
        for raw_type in ["group_announcement", "group_board"] {
            message.raw_type = raw_type.into();
            assert!(includes_message(&message, true, Some(&options)));
            assert!(!includes_message(&message, true, None));
            options.include_announcements = false;
            assert!(!includes_message(&message, false, Some(&options)));
            options.include_announcements = true;
        }
        message.raw_type = "2063".into();
        assert!(includes_message(&message, false, Some(&options)));
        options.include_recalled_messages = false;
        assert!(!includes_message(&message, false, Some(&options)));
        message.raw_type = "text".into();
        message.lifecycle = LifecycleState::Recalled;
        assert!(!includes_message(&message, false, Some(&options)));
        options.include_recalled_messages = true;
        assert!(includes_message(&message, false, Some(&options)));

        message.message_type = MessageType::Text;
        message.lifecycle = LifecycleState::DeletedAtSource;
        options.include_deleted_messages = false;
        assert!(!includes_message(&message, false, Some(&options)));
        options.include_deleted_messages = true;
        assert!(includes_message(&message, false, Some(&options)));
        options.message_types = Some(vec![MessageType::Image]);
        assert!(!includes_message(&message, false, Some(&options)));
        options.message_types = Some(vec![MessageType::Text]);
        assert!(includes_message(&message, false, Some(&options)));
        options.include_deleted_messages = false;
        assert!(!includes_message(&message, false, Some(&options)));
        options.message_types = Some(vec![]);
        message.lifecycle = LifecycleState::Unknown;
        assert!(!includes_message(&message, false, Some(&options)));
        options.message_types = None;
        assert!(includes_message(&message, false, Some(&options)));
    }

    #[test]
    fn partial_preferences_use_safe_compatible_defaults() {
        let options: ExportCustomization = serde_json::from_value(serde_json::json!({
            "fields": { "direction": false },
            "includeRecalledMessages": false,
            "messageTypes": [],
            "timezone": "asia_shanghai"
        }))
        .unwrap();
        assert!(!options.fields.direction);
        assert!(options.fields.body);
        assert!(!options.fields.technical_ids);
        assert!(options.include_system_messages);
        assert!(!options.include_recalled_messages);
        assert_eq!(options.message_types, Some(vec![]));
        assert_eq!(options.timezone, ExportTimezone::AsiaShanghai);
        assert!(
            serde_json::from_value::<ExportCustomization>(serde_json::json!({
                "includeRecalls": false
            }))
            .is_err()
        );
    }

    #[test]
    fn compact_formats_omit_unselected_fields_and_use_explicit_local_time() {
        let directory = tempfile::tempdir().unwrap();
        for format in [ExportFormat::Csv, ExportFormat::Html, ExportFormat::Md] {
            let mut package = crate::tests::fixture(format, false, true);
            package.messages[0].sent_at = "2026-09-16T16:30:00Z".parse().unwrap();
            package.customization = Some(compact_options());
            let path = directory.path().join(format!("{format:?}"));
            let result = export_package(&package, &path, &AtomicBool::new(false)).unwrap();
            assert_eq!(result.message_count, 1);
            assert_eq!(result.media_count, 1);
            let text = fs::read_to_string(path).unwrap();
            for excluded in [
                "internal-",
                "测试会话",
                "active",
                "incoming",
                "@附件",
                "仅群",
                "消息类型",
                "生命周期",
            ] {
                assert!(!text.contains(excluded), "{format:?} leaked {excluded}");
            }
            assert!(text.contains("测试成员"));
            assert!(text.contains("普通消息提及群公告"));
            if format == ExportFormat::Md {
                assert!(text.contains(r"2026\-09\-17 00:30:00\+08:00"));
                assert!(!text.contains("###"));
            } else {
                assert!(text.contains("2026-09-17 00:30:00+08:00"));
            }
            if format == ExportFormat::Html {
                assert!(text.contains("&lt;script&gt;"));
                assert!(!text.contains("<script>"));
                assert!(text.contains("default-src 'none'"));
                assert_eq!(
                    text.matches("<section").count(),
                    text.matches("</section>").count()
                );
            }
            if format == ExportFormat::Csv {
                let mut reader =
                    csv::Reader::from_reader(text.trim_start_matches('\u{feff}').as_bytes());
                assert_eq!(
                    reader.headers().unwrap(),
                    &csv::StringRecord::from(vec!["时间", "发送人", "正文"])
                );
                let row = reader.records().next().unwrap().unwrap();
                assert_eq!(&row[1], "'+测试成员");
                assert_eq!(&row[2], "'=1+1\n普通消息提及群公告 <script>");
            }
        }
    }

    #[test]
    fn deselecting_body_removes_content_and_technical_fields_are_explicit() {
        let directory = tempfile::tempdir().unwrap();
        for format in [ExportFormat::Csv, ExportFormat::Html, ExportFormat::Md] {
            let mut package = crate::tests::fixture(format, false, false);
            let mut options = compact_options();
            options.fields.body = false;
            options.fields.timestamp = false;
            options.fields.sender = false;
            options.fields.technical_ids = true;
            options.fields.quote = true;
            package.customization = Some(options);
            let path = directory.path().join(format!("{format:?}"));
            export_package(&package, &path, &AtomicBool::new(false)).unwrap();
            let text = fs::read_to_string(path).unwrap();
            assert!(!text.contains("普通消息"));
            assert!(!text.contains("测试成员"));
            assert!(!text.contains("2026"));
            assert!(!text.contains("internal-hash"));
            let unescaped = text.replace("\\-", "-").replace("\\_", "_");
            assert!(unescaped.contains("internal-message"));
            assert!(unescaped.contains("internal-quote"));
        }
    }

    #[test]
    fn markdown_fences_untrusted_body_without_changing_message_content() {
        let mut package = crate::tests::fixture(ExportFormat::Md, false, false);
        package.customization = Some(compact_options());
        let body = "```\n<script>alert(1)</script>\n![remote](https://example.invalid/image)\n````";
        package.messages[0].body_text = Some(body.into());
        let mut output = Vec::new();
        markdown_message(
            &mut output,
            &package,
            &ExportNames::new(&package),
            &package.messages[0],
            package.customization.as_ref().unwrap(),
        )
        .unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains(&format!("`````text\n{body}\n`````\n")));
    }

    #[test]
    fn json_preserves_complete_records_and_counts_after_filtering() {
        let directory = tempfile::tempdir().unwrap();
        let mut package = crate::tests::fixture(ExportFormat::Json, false, false);
        let baseline = complete_message_value(&package.messages[0]).unwrap();
        package.customization = Some(compact_options());
        let path = directory.path().join("archive.json");
        let result = export_package(&package, &path, &AtomicBool::new(false)).unwrap();
        assert_eq!(result.message_count, 1);
        let document: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert_eq!(document["conversations"][0]["messages"][0], baseline);
        // New preferences always use complete JSON, even if a stale caller
        // still sends the legacy simplify flag.
        package.simplify = true;
        let path = directory.path().join("legacy-flag.json");
        export_package(&package, &path, &AtomicBool::new(false)).unwrap();
        let document: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert_eq!(document["conversations"][0]["messages"][0], baseline);
        package.customization.as_mut().unwrap().message_types = Some(vec![]);
        let path = directory.path().join("empty.json");
        let result = export_package(&package, &path, &AtomicBool::new(false)).unwrap();
        assert_eq!(result.message_count, 0);
        assert_eq!(result.media_count, 0);
        let document: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert_eq!(document["conversations"], serde_json::json!([]));
    }
}
