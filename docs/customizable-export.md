# 自定义导出

在“导出记录”或“直接导出本机消息”中选择 Markdown、HTML、CSV 或 JSON，然后选择预设：

- **AI 精简**：紧凑排列，每个会话显示一次名称，保留时间、发送人、正文、附件与引用关联，默认排除其他系统消息、群公告、撤回和已删除记录。时间使用北京时间。
- **完整归档**：保留所有消息和阅读字段，使用详情排版。
- **自定义**：独立选择消息范围、显示字段、消息类型、排版和时区。

设置自动保存在当前浏览器/WebView 的本地存储中，两种导出入口共用。界面预览是虚构示例，实际文件以导出结果为准。

“包含消息”决定某条消息是否进入文件；“显示字段”只控制选中消息的展示。例如，关闭“状态”字段不会删除撤回记录，关闭“保留撤回记录”才会排除它们。普通系统消息、群公告和撤回通知分别控制。消息类型与这些条件同时满足才会导出；未选择任何类型时界面禁止导出。

可选字段为：会话、时间、发送人、消息类型、状态、收发方向、正文、附件、引用和技术标识。关闭正文或附件会让阅读格式缺少对应信息。引用只能显示来源中已有的关联，不能恢复源数据中缺失的引用；本功能不进行图片 OCR 或语音转写。

日期范围按所选时区解释，包含开始日和结束日全天。北京时间固定为 UTC+08:00。归档数据中的 UTC 时间不变。

JSON 用于重新导入：消息筛选仍生效，但始终保留完整的版本化字段和校验信息，不应用显示字段、紧凑排版或显示时区。Markdown/HTML/CSV 适用于阅读和分析。对原始归档的消息、附件及状态不作修改。

## API 兼容性

`POST /api/v1/exports`、异步导出和本机导出请求接受可选的 `customization` 对象。未提供时保留旧 `simplify` 行为；提供时，以明确配置为准，包括 JSON 始终使用可重新导入的结构。

```json
{
  "customization": {
    "includeSystemMessages": false,
    "includeRecalledMessages": false,
    "includeDeletedMessages": false,
    "includeAnnouncements": false,
    "messageTypes": ["text", "image", "audio", "video", "file", "link", "reply"],
    "layout": "compact",
    "timezone": "asia_shanghai",
    "fields": {
      "conversation": false,
      "sender": true,
      "timestamp": true,
      "messageType": false,
      "lifecycle": false,
      "direction": false,
      "body": true,
      "media": true,
      "quote": true,
      "technicalIds": false
    }
  }
}
```

`messageTypes` 省略或为 `null` 表示全部类型，空数组表示零条消息；布尔筛选缺省为 `true`。`fields` 缺省全部显示，只有 `technicalIds` 缺省为 `false`。API 默认排版为 `standard`、时区为 `utc`。界面 AI 预设会明确传递自己的配置。
