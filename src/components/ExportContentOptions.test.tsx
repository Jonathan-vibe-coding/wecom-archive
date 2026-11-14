import { fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ExportContentOptions } from "./ExportContentOptions";
import { EXPORT_SETTINGS_KEY, exportPreset } from "../domain/export-options";
import { useExportPreferences } from "../lib/use-export-preferences";
import type { ExportFormat } from "../domain/types";

function Settings({ format = "md" }: { format?: ExportFormat }) {
  const { preferences, selectPreset, updateCustomization } = useExportPreferences();
  return <ExportContentOptions format={format} customization={preferences.customization} preset={preferences.preset} onChange={updateCustomization} onPresetChange={selectPreset} />;
}

afterEach(() => { localStorage.removeItem(EXPORT_SETTINGS_KEY); vi.restoreAllMocks(); });

describe("export content settings", () => {
  it("keeps message filters separate from visible status and retains the custom draft across presets", () => {
    render(<Settings />);
    expect(screen.getByRole("button", { name: /AI 精简/ })).toHaveAttribute("aria-pressed", "true");
    fireEvent.click(screen.getByRole("checkbox", { name: "消息状态" }));
    expect(screen.getByRole("button", { name: /自定义/ })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("checkbox", { name: "撤回消息 / 通知" })).not.toBeChecked();
    fireEvent.click(screen.getByRole("button", { name: /完整归档/ }));
    expect(screen.getByRole("checkbox", { name: "撤回消息 / 通知" })).toBeChecked();
    fireEvent.click(screen.getByRole("button", { name: /自定义/ }));
    expect(screen.getByRole("checkbox", { name: "消息状态" })).toBeChecked();
    expect(screen.getByRole("checkbox", { name: "撤回消息 / 通知" })).not.toBeChecked();
  });

  it("persists custom display and filter settings without saving any archive data", () => {
    const first = render(<Settings />);
    fireEvent.click(screen.getByRole("checkbox", { name: "群公告" }));
    fireEvent.change(screen.getByRole("combobox", { name: "显示时区" }), { target: { value: "utc" } });
    const saved = JSON.parse(localStorage.getItem(EXPORT_SETTINGS_KEY)!);
    expect(saved).toMatchObject({ preset: "custom", customization: { ...exportPreset("ai"), includeAnnouncements: true, timezone: "utc" } });
    expect(Object.keys(saved)).toEqual(["preset", "customization", "customDraft"]);
    first.unmount();
    render(<Settings />);
    expect(screen.getByRole("checkbox", { name: "群公告" })).toBeChecked();
    expect(screen.getByRole("combobox", { name: "显示时区" })).toHaveValue("utc");
  });

  it("disables JSON display settings while retaining independent message filters", () => {
    render(<Settings format="json" />);
    expect(screen.getByText(/JSON 保留可重新导入的完整数据结构/)).toBeInTheDocument();
    for (const name of ["正文", "消息状态", "技术 ID"]) expect(screen.getByRole("checkbox", { name })).toBeDisabled();
    expect(screen.getByRole("combobox", { name: "日期筛选时区" })).toBeEnabled();
    expect(screen.getByRole("combobox", { name: "内容排版" })).toBeDisabled();
    expect(screen.getByRole("checkbox", { name: "其他系统消息" })).toBeEnabled();
    fireEvent.click(screen.getByRole("checkbox", { name: "群公告" }));
    expect(screen.getByRole("checkbox", { name: "群公告" })).toBeChecked();
  });

  it("warns when every message type is excluded and can restore all types", () => {
    render(<Settings />);
    fireEvent.click(screen.getByText(/按消息类型筛选/));
    fireEvent.click(screen.getByRole("button", { name: "清空类型" }));
    expect(screen.getByRole("alert")).toHaveTextContent("请至少选择一种类型");
    fireEvent.click(screen.getByRole("button", { name: "全选类型" }));
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("shows a safe synthetic preview and tolerates unavailable local storage", () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => { throw new Error("blocked"); });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => { throw new Error("blocked"); });
    render(<Settings />);
    const preview = screen.getByText("字段与排版示意（虚构消息）").closest("details")!;
    expect(within(preview).getByText(/2026-01-02 10:30:00/)).toBeInTheDocument();
    fireEvent.change(screen.getByRole("combobox", { name: "显示时区" }), { target: { value: "utc" } });
    expect(within(preview).getByText(/2026-01-02 02:30:00 UTC/)).toBeInTheDocument();
  });
});
