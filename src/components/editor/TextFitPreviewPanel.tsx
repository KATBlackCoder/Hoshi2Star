import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { Checkbox } from "@/components/ui/checkbox";
import { useGameFont } from "@/lib/gameFont";
import { outcomeLabel } from "@/lib/textFit";
import type {
  FitLineView,
  FitOverflow,
  FitOverride,
  FitPageView,
  LineBreaks,
  TextFitPreview,
} from "@/lib/types";
import {
  useActiveSegmentId,
  useActiveSegmentTargetText,
} from "@/stores/editor";
import { useActiveEngine, useProjectStore } from "@/stores/project";
import { useTextFitStore, useTextFitVersion } from "@/stores/textFit";

/** RPG Maker MZ default text colours (`\C[n]`, Window.png). */
const TEXT_COLORS = [
  "#ffffff",
  "#20a0d6",
  "#ff784c",
  "#66cc40",
  "#99ccff",
  "#ccc0ff",
  "#ffffa0",
  "#808080",
  "#c0c0c0",
  "#2080cc",
  "#ff3810",
  "#00a010",
  "#3e9ade",
  "#a098ff",
  "#ffcc20",
  "#000000",
  "#84aaff",
  "#ffff40",
  "#ff2020",
  "#202040",
  "#e08040",
  "#f0c040",
  "#4080c0",
  "#40c0f0",
  "#80ff80",
  "#c08080",
  "#8080ff",
  "#ff80ff",
  "#00a040",
  "#00e060",
  "#a060e0",
  "#c080ff",
];

const FACE_WIDTH = 144;

/** `Bitmap.drawText` baseline: `y + lineHeight − (lineHeight − size × 0.7) / 2`. */
function baseline(line: FitLineView, fontSize: number): number {
  return line.y + line.height - (line.height - fontSize * 0.7) / 2;
}

interface PageProps {
  page: FitPageView;
  preview: TextFitPreview;
  fontFamily: string;
  index: number;
  count: number;
}

function PreviewPage({ page, preview, fontFamily, index, count }: PageProps) {
  const { t } = useTranslation();
  const { innerWidth, innerHeight, limit, item } = preview;
  return (
    <figure className="space-y-1">
      {count > 1 && (
        <figcaption className="text-[10px] text-muted-foreground">
          {t("textFit.preview.page", { page: index + 1, count })}
        </figcaption>
      )}
      <svg
        viewBox={`0 0 ${innerWidth} ${innerHeight}`}
        className="w-full rounded-md"
        role="img"
        aria-label={t("textFit.preview.page", { page: index + 1, count })}
      >
        <rect width={innerWidth} height={innerHeight} fill="#141826" />
        {item.face && (
          <g>
            <rect
              x={4}
              y={0}
              width={FACE_WIDTH}
              height={innerHeight}
              fill="#262b3f"
            />
            <text
              x={4 + FACE_WIDTH / 2}
              y={innerHeight / 2}
              fontSize={18}
              fill="#8b93b0"
              textAnchor="middle"
            >
              {t("textFit.preview.portrait")}
            </text>
          </g>
        )}
        <line
          x1={limit}
          y1={0}
          x2={limit}
          y2={innerHeight}
          stroke="#f59e0b"
          strokeOpacity={0.55}
          strokeDasharray="6 6"
        />
        {page.lines.map((line, lineIndex) => (
          <g key={lineIndex} opacity={line.clipped ? 0.35 : 1}>
            {line.overflow && (
              <rect
                x={limit}
                y={line.y}
                width={Math.max(0, line.right - limit)}
                height={line.height}
                fill="#ef4444"
                fillOpacity={0.35}
              >
                <title>{t("textFit.preview.overflow")}</title>
              </rect>
            )}
            {line.runs.map((run, runIndex) =>
              run.icon ? (
                <rect
                  key={runIndex}
                  x={run.x + 2}
                  y={line.y + 2}
                  width={32}
                  height={32}
                  rx={4}
                  fill="#475569"
                />
              ) : (
                <text
                  key={runIndex}
                  x={run.x}
                  y={baseline(line, run.fontSize)}
                  fontSize={run.fontSize}
                  fontFamily={fontFamily}
                  fill={TEXT_COLORS[run.color] ?? TEXT_COLORS[0]}
                  xmlSpace="preserve"
                >
                  {run.text}
                </text>
              ),
            )}
          </g>
        ))}
      </svg>
    </figure>
  );
}

/**
 * The selected segment's whole message (or description) as the game will
 * draw it after text fitting: real font, portrait area, pages, overflow.
 */
export function TextFitPreviewPanel() {
  const { t } = useTranslation();
  const projectId = useProjectStore((s) => s.activeProjectId);
  const engine = useActiveEngine();
  const segmentId = useActiveSegmentId();
  const targetText = useActiveSegmentTargetText();
  const version = useTextFitVersion();
  const bump = useTextFitStore((s) => s.bump);
  const ready = projectId !== null && segmentId !== null && engine === "mv_mz";
  const fontFamily = useGameFont(ready ? projectId : null);

  const { data: preview, isLoading } = useQuery({
    queryKey: ["text-fit-preview", projectId, segmentId, targetText, version],
    queryFn: () =>
      invoke<TextFitPreview | null>("preview_text_fit", {
        projectId,
        segmentId,
        draftTarget: null,
      }),
    enabled: ready,
    staleTime: 0,
  });

  const choice = preview?.item.override ?? null;
  const setChoice = async (next: FitOverride) => {
    const isDefault =
      next.lineBreaks === null && next.overflow === null && !next.disabled;
    await invoke("set_text_fit_override", {
      projectId,
      segmentId,
      choice: isDefault ? null : next,
    });
    bump();
  };
  const current: FitOverride = choice ?? {
    lineBreaks: null,
    overflow: null,
    disabled: false,
  };

  return (
    <div className="flex h-full flex-col">
      <div className="shrink-0 border-b px-3 py-2 text-[10px] font-semibold uppercase tracking-[0.12em]">
        {t("textFit.preview.title")}
      </div>
      <div className="flex-1 space-y-3 overflow-y-auto p-2">
        {!ready ? (
          <p className="py-4 text-center text-xs text-muted-foreground">
            {engine && engine !== "mv_mz"
              ? t("textFit.preview.mzOnly")
              : t("textFit.preview.empty")}
          </p>
        ) : isLoading ? (
          <p className="py-4 text-center text-xs text-muted-foreground">
            {t("textFit.preview.loading")}
          </p>
        ) : !preview ? (
          <p className="py-4 text-center text-xs text-muted-foreground">
            {t("textFit.preview.notFitted")}
          </p>
        ) : (
          <>
            <div className="space-y-0.5 text-xs">
              <p className="font-medium">
                {outcomeLabel(t, preview.item.fitted.outcome)}
              </p>
              {!preview.enabled && (
                <p className="text-muted-foreground">
                  {t("textFit.preview.off")}
                </p>
              )}
            </div>
            {preview.item.fitted.pages.map((page, index) => (
              <PreviewPage
                key={index}
                page={page}
                preview={preview}
                fontFamily={fontFamily ?? "monospace"}
                index={index}
                count={preview.item.fitted.pages.length}
              />
            ))}
            <div className="space-y-2 rounded-lg border p-2 text-xs">
              <label className="flex items-center justify-between gap-2">
                <span>{t("textFit.preview.lineBreaks")}</span>
                <select
                  className="h-8 rounded-md border bg-background px-1.5"
                  value={current.lineBreaks ?? ""}
                  disabled={current.disabled}
                  onChange={(e) =>
                    void setChoice({
                      ...current,
                      lineBreaks: (e.target.value || null) as LineBreaks | null,
                    })
                  }
                >
                  <option value="">{t("textFit.preview.default")}</option>
                  <option value="reflow">
                    {t("textFit.lineBreaksReflow")}
                  </option>
                  <option value="keep">{t("textFit.lineBreaksKeep")}</option>
                </select>
              </label>
              <label className="flex items-center justify-between gap-2">
                <span>{t("textFit.preview.overflowLabel")}</span>
                <select
                  className="h-8 rounded-md border bg-background px-1.5"
                  value={current.overflow ?? ""}
                  disabled={current.disabled}
                  onChange={(e) =>
                    void setChoice({
                      ...current,
                      overflow: (e.target.value || null) as FitOverflow | null,
                    })
                  }
                >
                  <option value="">{t("textFit.preview.default")}</option>
                  {!preview.item.description && (
                    <option value="new_page">
                      {t("textFit.overflowNewPage")}
                    </option>
                  )}
                  <option value="shrink">{t("textFit.overflowShrink")}</option>
                  <option value="leave">{t("textFit.overflowLeave")}</option>
                </select>
              </label>
              <label className="flex items-center gap-2">
                <Checkbox
                  checked={current.disabled}
                  onCheckedChange={(value) =>
                    void setChoice({ ...current, disabled: value === true })
                  }
                />
                <span>{t("textFit.preview.asTranslated")}</span>
              </label>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
