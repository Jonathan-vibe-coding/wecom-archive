import { Check, Download, FileDown, LoaderCircle } from "lucide-react";
import { useState } from "react";
import type { ExportFormat } from "../domain/types";
import { createLocalExport, ServerApiError, type ExportOrder, type ServerExportResult } from "../lib/server-api";
import { ExportPresentationOptions } from "./ExportPresentationOptions";
import { ExportOrderOptions } from "./ExportOrderOptions";
import { ExportDirectoryButton } from "./ExportDirectoryButton";
import { CustomSourceRootDialog } from "./Overlays";
import { ExportContentOptions, type ExportContentOptionsProps } from "./ExportContentOptions";
import { hasNoExportMessageTypes } from "../domain/export-options";

interface LocalExportPageProps {
  token: string;
  onCompleted: (result: ServerExportResult) => void;
  onFailed: (error: unknown) => void;
  contentOptions: Omit<ExportContentOptionsProps, "format" | "disabled">;
}

export function LocalExportPage({ token, onCompleted, onFailed, contentOptions }: LocalExportPageProps) {
  const [format, setFormat] = useState<ExportFormat>("md");
  const [dataRedaction, setDataRedaction] = useState(true);
  const [pretty, setPretty] = useState(true);
  const [conversationOrder, setConversationOrder] = useState<ExportOrder>("descending");
  const [messageOrder, setMessageOrder] = useState<ExportOrder>("descending");
  const [busy, setBusy] = useState(false);
  const [customRootIssue, setCustomRootIssue] = useState<{ message: string; attemptedRoot?: string } | null>(null);

  const submit = async (sourceRoot?: string) => {
    if (busy || hasNoExportMessageTypes(contentOptions.customization)) return;
    setBusy(true);
    try {
      onCompleted(await createLocalExport(token, { format, sourceRoot, dataRedaction, simplify: false, pretty, conversationOrder, messageOrder, customization: contentOptions.customization }));
      setCustomRootIssue(null);
    } catch (error) {
      onFailed(error);
      if (error instanceof ServerApiError && error.code === "LOCAL_SOURCE_NOT_FOUND") {
        setCustomRootIssue({ message: error.message, attemptedRoot: sourceRoot });
      } else {
        setCustomRootIssue(null);
      }
    } finally {
      setBusy(false);
    }
  };

  return <main className="full-page local-export-page">
    <section className="local-export-card">
      <div className="local-export-card-heading"><span className="local-export-icon"><FileDown size={22} /></span><div><h2>快速导出本机记录</h2></div><ExportDirectoryButton token={token} onFailed={onFailed} /></div>
      <div className="local-export-options">
        <fieldset disabled={busy}><legend>导出格式</legend><div className="format-grid">{(["json", "csv", "html", "md"] as ExportFormat[]).map((item) => <button className={format === item ? "selected" : ""} key={item} type="button" onClick={() => setFormat(item)}><span>{format === item && <Check size={15} />}{item.toUpperCase()}</span></button>)}</div></fieldset>
        <div className="local-export-config-row">
          <fieldset className="local-export-config-options"><legend>导出配置</legend><ExportPresentationOptions dataRedaction={dataRedaction} simplify={false} pretty={pretty} onDataRedactionChange={setDataRedaction} onSimplifyChange={() => {}} onPrettyChange={setPretty} disabled={busy} hideSimplify /></fieldset>
          <ExportOrderOptions conversationOrder={conversationOrder} messageOrder={messageOrder} onConversationOrderChange={setConversationOrder} onMessageOrderChange={setMessageOrder} disabled={busy} />
        </div>
        <ExportContentOptions {...contentOptions} format={format} disabled={busy} />
      </div>
      <div className="local-export-actions"><button className="primary-button" type="button" disabled={busy || hasNoExportMessageTypes(contentOptions.customization)} onClick={() => void submit()}>{busy ? <><LoaderCircle className="spin" size={17} />正在读取并导出…</> : <><Download size={17} />开始导出</>}</button></div>
    </section>
    {customRootIssue && !busy && <CustomSourceRootDialog message={customRootIssue.message} initialRoot={customRootIssue.attemptedRoot ?? ""} onClose={() => setCustomRootIssue(null)} onRetry={(sourceRoot) => { void submit(sourceRoot); }} />}
  </main>;
}
