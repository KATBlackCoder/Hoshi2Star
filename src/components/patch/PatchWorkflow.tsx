import {
  ArchiveRestore,
  BookOpenText,
  Languages,
  PackageCheck,
  ScanSearch,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";
import { useProjectStore } from "@/stores/project";
import { useUiStore } from "@/stores/ui";

const STEPS = [
  { id: "extraction", icon: ArchiveRestore },
  { id: "terms", icon: BookOpenText },
  { id: "translation", icon: Languages },
  { id: "review", icon: ScanSearch },
  { id: "export", icon: PackageCheck },
] as const;

export function PatchWorkflow({
  onExport,
  exporting = false,
}: {
  onExport: () => void;
  exporting?: boolean;
}) {
  const { t } = useTranslation();
  const activeProjectId = useProjectStore((state) => state.activeProjectId);
  const setMode = useUiStore((state) => state.setMode);
  const openInspector = useUiStore((state) => state.openInspector);

  function activate(id: (typeof STEPS)[number]["id"]) {
    if (id === "extraction") setMode("library");
    else if (id === "terms") setMode("terminology");
    else if (id === "translation") setMode("patch");
    else if (id === "review") openInspector("qa");
    else onExport();
  }

  return (
    <nav
      aria-label={t("workflow.title")}
      className="shrink-0 overflow-x-auto border-b bg-card/55 px-2 py-1.5 [scrollbar-width:none]"
    >
      <ol className="mx-auto grid min-w-[36rem] max-w-5xl grid-cols-5 gap-1">
        {STEPS.map(({ id, icon: Icon }, index) => {
          const current = id === "translation";
          const decision = id === "export";
          const disabled = !activeProjectId || (decision && exporting);
          return (
            <li key={id} className="min-w-0">
              <button
                id={`patch-gate-${id}`}
                type="button"
                className={cn(
                  "group flex min-h-11 w-full min-w-0 items-center gap-2 rounded-lg px-2 text-left transition-[background-color,box-shadow,color,transform] active:scale-[0.96] disabled:pointer-events-none disabled:opacity-45",
                  current && "bg-primary/10 text-primary shadow-[var(--shadow-surface)]",
                  decision && "shrine-edge text-seal hover:bg-seal/10",
                  !current && !decision && "text-muted-foreground hover:bg-accent/60 hover:text-foreground",
                )}
                aria-current={current ? "step" : undefined}
                disabled={disabled}
                onClick={() => activate(id)}
              >
                <span className="grid size-7 shrink-0 place-items-center rounded-md bg-background/75 font-mono text-[10px] shadow-[var(--shadow-surface)]">
                  {index + 1}
                </span>
                <span className="min-w-0">
                  <span className="block truncate text-xs font-semibold">
                    {t(`workflow.${id}`)}
                  </span>
                  <span className="block truncate text-[10px] text-muted-foreground">
                    {t(`workflow.${activeProjectId ? "ready" : "waiting"}`)}
                  </span>
                </span>
                <Icon className="ml-auto size-3.5 shrink-0 opacity-65" aria-hidden="true" />
              </button>
            </li>
          );
        })}
      </ol>
    </nav>
  );
}
