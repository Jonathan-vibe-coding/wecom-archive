import { describe, expect, it } from "vitest";
import { defaultExportPreferences, exportPreset, parseExportPreferences } from "./export-options";

describe("saved export settings", () => {
  it.each([null, "{", "null", "[]", '{"preset":"custom"}', '{"preset":"unknown"}'])("recovers from invalid preferences %s", (serialized) => {
    expect(parseExportPreferences(serialized)).toEqual(defaultExportPreferences());
  });

  it("round trips custom settings and an intentionally empty message type list", () => {
    const customization = { ...exportPreset("complete"), messageTypes: [], timezone: "utc" as const };
    const preferences = { preset: "custom", customization, customDraft: customization };
    expect(parseExportPreferences(JSON.stringify(preferences))).toEqual(preferences);
  });

  it("rejects invalid fields and message type values without retaining arbitrary data", () => {
    const preferences = defaultExportPreferences();
    const unsafe = { ...preferences, customDraft: { ...preferences.customDraft, messageTypes: ["not-a-type"] }, token: "synthetic-secret" };
    expect(parseExportPreferences(JSON.stringify(unsafe))).toEqual(defaultExportPreferences());
    const withExtra = parseExportPreferences(JSON.stringify({ ...preferences, token: "synthetic-secret" }));
    expect(JSON.stringify(withExtra)).not.toContain("synthetic-secret");
  });
});
