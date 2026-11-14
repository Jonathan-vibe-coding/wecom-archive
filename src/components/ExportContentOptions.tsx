import { exportFieldLabels, exportMessageTypeLabels, exportPreview, hasNoExportMessageTypes, type ExportCustomization, type ExportPreset } from "../domain/export-options";
import type { ExportFormat } from "../domain/types";

export interface ExportContentOptionsProps {
  format: ExportFormat;
  customization: ExportCustomization;
  preset: ExportPreset;
  onChange: (value: ExportCustomization) => void;
  onPresetChange: (preset: ExportPreset) => void;
  disabled?: boolean;
}

export function ExportContentOptions({ format, customization, preset, onChange, onPresetChange, disabled }: ExportContentOptionsProps) {
  const json = format === "json";
  const selectedTypes = customization.messageTypes ?? exportMessageTypeLabels.map(([type]) => type);
  const noTypes = hasNoExportMessageTypes(customization);
  return <section className="export-content-options" aria-label="导出内容设置">
    <div className="export-content-heading"><strong>导出内容</strong><small>设置会记住，并用于两个导出入口</small></div>
    <div className="export-preset-grid" role="group" aria-label="导出预设">
      {([["ai", "AI 精简", "保留时间、发送人和内容"], ["complete", "完整归档", "保留全部消息和字段"], ["custom", "自定义", "调整并保存自己的设置"]] as const).map(([value, label, hint]) => <button key={value} type="button" disabled={disabled} aria-pressed={preset === value} className={preset === value ? "selected" : ""} onClick={() => onPresetChange(value)}><strong>{label}</strong><small>{hint}</small></button>)}
    </div>
    <fieldset disabled={disabled} className="export-content-filters"><legend>保留哪些消息</legend>
      <div className="export-checkbox-grid">
        {([["includeSystemMessages", "其他系统消息"], ["includeRecalledMessages", "撤回消息 / 通知"], ["includeDeletedMessages", "源端已删除"], ["includeAnnouncements", "群公告"]] as const).map(([key, label]) => <label key={key}><input type="checkbox" checked={customization[key]} onChange={(event) => onChange({ ...customization, [key]: event.target.checked })} />{label}</label>)}
      </div>
      <small className="export-option-hint">这些选项各自生效。隐藏“消息状态”字段不会删除对应消息；仅导出本机已有内容。</small>
      <details className="export-type-picker"><summary>按消息类型筛选 · 已选 {selectedTypes.length} / {exportMessageTypeLabels.length}</summary><div className="export-type-actions"><button type="button" disabled={disabled} onClick={() => onChange({ ...customization, messageTypes: undefined })}>全选类型</button><button type="button" disabled={disabled} onClick={() => onChange({ ...customization, messageTypes: [] })}>清空类型</button></div><div className="export-checkbox-grid">{exportMessageTypeLabels.map(([type, label]) => <label key={type}><input type="checkbox" checked={selectedTypes.includes(type)} onChange={(event) => { const next = event.target.checked ? [...selectedTypes, type] : selectedTypes.filter((item) => item !== type); onChange({ ...customization, messageTypes: next.length === exportMessageTypeLabels.length ? undefined : next }); }} />{label}</label>)}</div><small className="export-option-hint">类型筛选与上面的消息筛选同时生效。</small></details>
      {noTypes && <p className="export-options-warning" role="alert">未选择消息类型，无法导出消息。请至少选择一种类型。</p>}
    </fieldset>
    {json && <p className="export-options-note">JSON 保留可重新导入的完整数据结构，只应用消息筛选。需要精简字段供 AI 分析时，请选择 MD、HTML 或 CSV。</p>}
    <fieldset disabled={disabled || json} className="export-content-fields"><legend>展示哪些字段</legend><div className="export-checkbox-grid">{exportFieldLabels.map(([key, label]) => <label key={key}><input type="checkbox" checked={customization.fields[key]} onChange={(event) => onChange({ ...customization, fields: { ...customization.fields, [key]: event.target.checked } })} />{label}</label>)}</div>
      {format === "csv" && <small className="export-option-hint">CSV 按所选字段生成列；紧凑排版仅用于 MD 和 HTML。</small>}
      {!json && !Object.values(customization.fields).some(Boolean) && <p className="export-options-warning">未选择展示字段，导出文件将不显示消息内容。</p>}
    </fieldset>
    <div className="export-display-options"><label>排版<select aria-label="内容排版" value={customization.layout} disabled={disabled || json || format === "csv"} onChange={(event) => onChange({ ...customization, layout: event.target.value as ExportCustomization["layout"] })}><option value="compact">紧凑对话</option><option value="standard">逐条详情</option></select></label><label>{json ? "日期筛选时区（JSON 内 UTC 时间不变）" : "显示与日期筛选时区"}<select aria-label={json ? "日期筛选时区" : "显示时区"} value={customization.timezone} disabled={disabled} onChange={(event) => onChange({ ...customization, timezone: event.target.value as ExportCustomization["timezone"] })}><option value="asia_shanghai">北京时间 · UTC+8</option><option value="utc">UTC</option></select></label></div>
    {!json && <details className="export-preview" open><summary>字段与排版示意（虚构消息）</summary><pre>{exportPreview(customization)}</pre><small className="export-option-hint">示意用于确认字段；实际文件按所选格式排版，消息以筛选结果为准。</small></details>}
  </section>;
}
