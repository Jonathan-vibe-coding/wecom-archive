import { useEffect, useState } from "react";
import { defaultExportPreferences, EXPORT_SETTINGS_KEY, exportPreset, parseExportPreferences, type ExportCustomization, type ExportPreset } from "../domain/export-options";

export function useExportPreferences() {
  const [preferences, setPreferences] = useState(() => {
    try { return parseExportPreferences(localStorage.getItem(EXPORT_SETTINGS_KEY)); }
    catch { return defaultExportPreferences(); }
  });
  useEffect(() => {
    try { localStorage.setItem(EXPORT_SETTINGS_KEY, JSON.stringify(preferences)); }
    catch { /* Export remains available when browser storage is unavailable. */ }
  }, [preferences]);
  const selectPreset = (preset: ExportPreset) => setPreferences((current) => ({
    ...current, preset, customization: preset === "custom" ? current.customDraft : exportPreset(preset),
  }));
  const updateCustomization = (customization: ExportCustomization) => setPreferences({ preset: "custom", customization, customDraft: customization });
  return { preferences, selectPreset, updateCustomization };
}
