import type { MessageType } from "./types";

export interface ExportFields {
  conversation: boolean;
  sender: boolean;
  timestamp: boolean;
  messageType: boolean;
  lifecycle: boolean;
  direction: boolean;
  body: boolean;
  media: boolean;
  quote: boolean;
  technicalIds: boolean;
}

export interface ExportCustomization {
  fields: ExportFields;
  includeSystemMessages: boolean;
  includeRecalledMessages: boolean;
  includeDeletedMessages: boolean;
  includeAnnouncements: boolean;
  messageTypes?: MessageType[];
  layout: "standard" | "compact";
  timezone: "utc" | "asia_shanghai";
}

export type ExportPreset = "ai" | "complete" | "custom";
export interface ExportPreferences {
  preset: ExportPreset;
  customization: ExportCustomization;
  customDraft: ExportCustomization;
}

export const EXPORT_SETTINGS_KEY = "wecom-export-preferences.v1";
export const exportFieldLabels: ReadonlyArray<readonly [keyof ExportFields, string]> = [
  ["timestamp", "时间"], ["sender", "发送人"], ["body", "正文"],
  ["media", "附件信息"], ["quote", "引用关联"], ["conversation", "会话名称"],
  ["messageType", "消息类型"], ["lifecycle", "消息状态"], ["direction", "收发方向"],
  ["technicalIds", "技术 ID"],
];
export const exportMessageTypeLabels: ReadonlyArray<readonly [MessageType, string]> = [
  ["text", "文本"], ["image", "图片"], ["audio", "语音"], ["video", "视频"],
  ["file", "文件"], ["link", "链接"], ["reply", "回复"], ["system", "系统"], ["unsupported", "其他"],
];

export function exportPreset(preset: "ai" | "complete"): ExportCustomization {
  const complete = preset === "complete";
  return {
    fields: {
      conversation: true, sender: true, timestamp: true, messageType: complete,
      lifecycle: complete, direction: complete, body: true, media: true, quote: true,
      technicalIds: complete,
    },
    includeSystemMessages: complete, includeRecalledMessages: complete,
    includeDeletedMessages: complete, includeAnnouncements: complete,
    layout: complete ? "standard" : "compact", timezone: "asia_shanghai",
  };
}

export function defaultExportPreferences(): ExportPreferences {
  return { preset: "ai", customization: exportPreset("ai"), customDraft: exportPreset("ai") };
}

function record(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function parseCustomization(value: unknown): ExportCustomization | undefined {
  if (!record(value) || !record(value.fields)) return undefined;
  const fields = {} as ExportFields;
  for (const [key] of exportFieldLabels) {
    if (typeof value.fields[key] !== "boolean") return undefined;
    fields[key] = value.fields[key];
  }
  const filterKeys = ["includeSystemMessages", "includeRecalledMessages", "includeDeletedMessages", "includeAnnouncements"] as const;
  for (const key of filterKeys) if (typeof value[key] !== "boolean") return undefined;
  if (value.layout !== "standard" && value.layout !== "compact") return undefined;
  if (value.timezone !== "utc" && value.timezone !== "asia_shanghai") return undefined;
  const messageTypes = value.messageTypes;
  if (messageTypes !== undefined && (!Array.isArray(messageTypes) || messageTypes.some((type) => !exportMessageTypeLabels.some(([key]) => key === type)))) return undefined;
  return {
    fields,
    includeSystemMessages: value.includeSystemMessages as boolean,
    includeRecalledMessages: value.includeRecalledMessages as boolean,
    includeDeletedMessages: value.includeDeletedMessages as boolean,
    includeAnnouncements: value.includeAnnouncements as boolean,
    ...(messageTypes === undefined ? {} : { messageTypes: [...new Set(messageTypes as MessageType[])] }),
    layout: value.layout, timezone: value.timezone,
  };
}

export function parseExportPreferences(serialized: string | null): ExportPreferences {
  try {
    const value: unknown = JSON.parse(serialized ?? "null");
    if (!record(value) || !["ai", "complete", "custom"].includes(String(value.preset))) return defaultExportPreferences();
    const customization = parseCustomization(value.customization);
    const customDraft = parseCustomization(value.customDraft);
    if (!customization || !customDraft) return defaultExportPreferences();
    // Built-in presets stay consistent with their label after future app updates.
    const preset = value.preset as ExportPreset;
    return { preset, customization: preset === "custom" ? customization : exportPreset(preset), customDraft };
  } catch {
    return defaultExportPreferences();
  }
}

export function hasNoExportMessageTypes(customization: ExportCustomization): boolean {
  return customization.messageTypes?.length === 0;
}

/** A synthetic illustration of the selected fields, never actual archive data. */
export function exportPreview(customization: ExportCustomization): string {
  const { fields, timezone, layout } = customization;
  const time = timezone === "asia_shanghai" ? "2026-01-02 10:30:00 +08:00" : "2026-01-02 02:30:00 UTC";
  const values: Record<keyof ExportFields, string> = {
    conversation: "示例项目群", timestamp: time, sender: "小林", body: "测试已完成，明天一起确认结果。",
    messageType: "文本", lifecycle: "active", direction: "incoming", media: "附件：测试报告.pdf",
    quote: "引用消息：example-message-0", technicalIds: "message_id: example-message-1",
  };
  if (!Object.values(fields).some(Boolean)) return "未选择展示字段。";
  if (layout === "standard") return exportFieldLabels.filter(([key]) => fields[key]).map(([key, label]) => `${label}：${values[key]}`).join("\n");
  const header = (["conversation", "timestamp", "sender", "messageType", "lifecycle", "direction", "technicalIds"] as const).filter((key) => fields[key]).map((key) => values[key]).join(" · ");
  return [header, ...(["body", "quote", "media"] as const).filter((key) => fields[key]).map((key) => values[key])].filter(Boolean).join("\n");
}
