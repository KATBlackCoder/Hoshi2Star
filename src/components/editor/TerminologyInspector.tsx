import { invoke } from "@tauri-apps/api/core";
import { useQuery } from "@tanstack/react-query";
import { ArrowUpRight, BookMarked } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { useTerminologyUiStore } from "@/features/terminology/terminologyUiStore";
import type { SegmentTerminologyRule } from "@/lib/types";
import { useActiveSegmentId } from "@/stores/editor";
import { useUiStore } from "@/stores/ui";

export function TerminologyInspector({
  projectId,
  langPair,
}: {
  projectId: string | null;
  langPair: string;
}) {
  const { t } = useTranslation();
  const segmentId = useActiveSegmentId();
  const languageParts = langPair.split("-");
  const targetLanguage = languageParts[languageParts.length - 1] || "en";
  const setMode = useUiStore((state) => state.setMode);
  const setSearch = useTerminologyUiStore((state) => state.setSearch);
  const setScope = useTerminologyUiStore((state) => state.setScope);
  const rules = useQuery({
    queryKey: ["segment-terminology", segmentId, targetLanguage],
    queryFn: () =>
      invoke<SegmentTerminologyRule[]>("get_segment_terminology", {
        segmentId: segmentId!,
        targetLanguage,
      }),
    enabled: Boolean(projectId && segmentId),
  });

  const openLibrary = (source?: string) => {
    setScope(projectId ? "project" : "global");
    setSearch(source ?? "");
    setMode("terminology");
  };

  return (
    <div className="flex h-full flex-col overflow-hidden">
      <div className="flex min-h-10 shrink-0 items-center gap-2 border-b px-3 py-2">
        <BookMarked className="size-3.5 text-muted-foreground" aria-hidden="true" />
        <span className="text-[10px] font-semibold uppercase tracking-[0.12em] text-muted-foreground/80">
          {t("terminologyInspector.title")}
        </span>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="ml-auto h-7 gap-1.5"
          onClick={() => openLibrary()}
        >
          {t("terminologyInspector.openLibrary")}
          <ArrowUpRight className="size-3" aria-hidden="true" />
        </Button>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto p-2">
        {!segmentId ? (
          <p className="py-5 text-center text-xs leading-5 text-muted-foreground">
            {t("terminologyInspector.selectSegment")}
          </p>
        ) : rules.isLoading ? (
          <p className="py-5 text-center text-xs text-muted-foreground" aria-live="polite">
            {t("terminologyInspector.loading")}
          </p>
        ) : (rules.data?.length ?? 0) === 0 ? (
          <p className="py-5 text-center text-xs leading-5 text-muted-foreground">
            {t("terminologyInspector.empty")}
          </p>
        ) : (
          <ul className="space-y-2">
            {rules.data?.map((rule) => (
              <li key={rule.entryId}>
                <button
                  type="button"
                  className="w-full rounded-xl bg-muted/45 p-3 text-left shadow-[inset_0_0_0_1px_rgb(255_255_255/0.04)] transition-[background-color,transform] hover:bg-muted/70 active:scale-[0.99] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  onClick={() => openLibrary(rule.source)}
                  aria-label={t("terminologyInspector.openTerm", {
                    source: rule.source,
                  })}
                >
                  <span className="flex items-baseline justify-between gap-3">
                    <span lang={langPair.split("-")[0]} className="font-medium">
                      {rule.source}
                    </span>
                    <span lang={targetLanguage} className="text-sm text-primary">
                      {rule.target}
                    </span>
                  </span>
                  <span className="mt-2 flex flex-wrap gap-1">
                    <Badge variant="secondary">{rule.semanticType}</Badge>
                    <Badge variant="outline">{rule.partOfSpeech}</Badge>
                    <Badge variant="outline">{rule.reviewStatus}</Badge>
                    <Badge variant="outline">{rule.enforcement}</Badge>
                  </span>
                  {rule.acceptedTargets.length > 0 && (
                    <span className="mt-2 block text-xs text-muted-foreground">
                      {t("terminologyInspector.variants", {
                        variants: rule.acceptedTargets.join(" · "),
                      })}
                    </span>
                  )}
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
