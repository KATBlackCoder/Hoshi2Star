import { useTranslation } from "react-i18next";
import type { WolfExportPreflight } from "@/lib/types";

// ---------------------------------------------------------------------------
// ExportWolfNotes — what a Wolf RPG export will do, in the export confirm
// ---------------------------------------------------------------------------

interface ExportWolfNotesProps {
  notes: WolfExportPreflight;
}

export function ExportWolfNotes({ notes }: ExportWolfNotesProps) {
  const { t } = useTranslation();
  const lines: string[] = [];
  if (notes.accentedTexts > 0) {
    lines.push(
      t(notes.shiftJis ? "exportWolf.folded" : "exportWolf.fontAccents", {
        count: notes.accentedTexts,
      }),
    );
  }
  if (lines.length === 0 && !notes.pathWarning) return null;

  return (
    <div className="rounded-lg border p-3 text-left text-xs">
      {notes.pathWarning && (
        <p role="alert" className="mb-2 font-medium text-destructive">
          {t("exportWolf.pathWarning")}
        </p>
      )}
      {lines.length > 0 && (
        <ul className="list-disc space-y-1 pl-4">
          {lines.map((line) => (
            <li key={line}>{line}</li>
          ))}
        </ul>
      )}
    </div>
  );
}
