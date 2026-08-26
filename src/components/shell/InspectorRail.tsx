import {
  BookMarked,
  Database,
  PanelRightClose,
  ShieldCheck,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { GlossaryPanel } from "@/components/editor/GlossaryPanel";
import { QAPanel } from "@/components/editor/QAPanel";
import { TMPanel } from "@/components/editor/TMPanel";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { useUiStore, type InspectorTab } from "@/stores/ui";

interface InspectorRailProps {
  projectId: string | null;
  langPair: string;
  sourceText: string | null;
  targetText: string | null;
}

const TABS: Array<{
  id: InspectorTab;
  labelKey: string;
  icon: typeof Database;
}> = [
  { id: "tm", labelKey: "inspector.tm", icon: Database },
  { id: "qa", labelKey: "inspector.qa", icon: ShieldCheck },
  { id: "glossary", labelKey: "inspector.glossary", icon: BookMarked },
];

export function InspectorRail({
  projectId,
  langPair,
  sourceText,
  targetText,
}: InspectorRailProps) {
  const { t } = useTranslation();
  const inspectorOpen = useUiStore((state) => state.inspectorOpen);
  const inspectorTab = useUiStore((state) => state.inspectorTab);
  const toggleInspector = useUiStore((state) => state.toggleInspector);
  const setInspectorTab = useUiStore((state) => state.setInspectorTab);

  const selectTab = (tab: InspectorTab) => {
    setInspectorTab(tab);
    if (!inspectorOpen) toggleInspector();
  };

  if (!inspectorOpen) {
    return (
      <aside
        aria-label={t("inspector.title")}
        className="flex w-12 shrink-0 flex-col items-center gap-1 bg-card/40 py-2 shadow-[-1px_0_0_rgb(255_255_255/0.06)]"
      >
        {TABS.map(({ id, labelKey, icon: Icon }) => (
          <Button
            key={id}
            type="button"
            variant="ghost"
            size="icon"
            className={cn(
              "h-10 w-10 text-muted-foreground",
              id === inspectorTab && "bg-accent text-foreground",
            )}
            aria-label={t(labelKey)}
            aria-pressed={id === inspectorTab}
            onClick={() => selectTab(id)}
          >
            <Icon className="h-4 w-4" />
          </Button>
        ))}
      </aside>
    );
  }

  return (
    <aside
      aria-label={t("inspector.title")}
      className="flex w-[clamp(20rem,25vw,27.5rem)] shrink-0 flex-col overflow-hidden bg-card/55 shadow-[-1px_0_0_rgb(255_255_255/0.08),-8px_0_24px_rgb(0_0_0/0.08)] backdrop-blur-sm"
    >
      <div className="flex min-h-12 shrink-0 items-center gap-1 px-1.5 shadow-[0_1px_0_rgb(255_255_255/0.06)]">
        <div
          role="tablist"
          aria-label={t("inspector.tools")}
          className="grid min-w-0 flex-1 grid-cols-3 gap-1"
        >
          {TABS.map(({ id, labelKey, icon: Icon }) => (
            <button
              key={id}
              id={`inspector-tab-${id}`}
              type="button"
              role="tab"
              aria-selected={id === inspectorTab}
              aria-controls={`inspector-panel-${id}`}
              className={cn(
                "flex min-h-10 min-w-0 items-center justify-center gap-1.5 rounded-lg px-2 text-xs font-medium transition-[background-color,color,transform] active:scale-[0.96]",
                id === inspectorTab
                  ? "bg-accent text-foreground shadow-[var(--shadow-surface)]"
                  : "text-muted-foreground hover:bg-accent/55 hover:text-foreground",
              )}
              onClick={() => selectTab(id)}
            >
              <Icon className="h-3.5 w-3.5 shrink-0" />
              <span className="truncate">{t(labelKey)}</span>
            </button>
          ))}
        </div>

        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="h-10 w-10 shrink-0 text-muted-foreground"
          aria-label={t("inspector.close")}
          onClick={toggleInspector}
        >
          <PanelRightClose className="h-4 w-4" />
        </Button>
      </div>

      <div
        id={`inspector-panel-${inspectorTab}`}
        role="tabpanel"
        aria-labelledby={`inspector-tab-${inspectorTab}`}
        className="min-h-0 flex-1 overflow-hidden"
      >
        {inspectorTab === "tm" ? (
          <TMPanel />
        ) : inspectorTab === "qa" ? (
          <QAPanel sourceText={sourceText} targetText={targetText} />
        ) : (
          <GlossaryPanel projectId={projectId} langPair={langPair} />
        )}
      </div>
    </aside>
  );
}
