import {
  BookOpenText,
  Images,
  Languages,
  Library,
  Play,
  Settings,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { type AppMode, useUiStore } from "@/stores/ui";
import { HoshiyomiMark } from "@/components/brand/HoshiyomiMark";

const MODES = [
  { id: "library", icon: Library },
  { id: "patch", icon: Languages },
  { id: "terminology", icon: BookOpenText },
  { id: "player", icon: Play },
  { id: "images", icon: Images },
] as const;

export function ModeNavigation({
  onOpenSettings,
}: {
  onOpenSettings: () => void;
}) {
  const { t } = useTranslation();
  const mode = useUiStore((state) => state.mode);
  const setMode = useUiStore((state) => state.setMode);

  function moveFrom(current: AppMode, delta: number) {
    const index = MODES.findIndex((item) => item.id === current);
    const next = MODES[(index + delta + MODES.length) % MODES.length];
    setMode(next.id);
    document.getElementById(`mode-${next.id}`)?.focus();
  }

  return (
    <header className="relative z-20 flex min-h-16 min-w-0 shrink-0 items-center gap-3 border-b bg-background/88 px-3 backdrop-blur-md">
      <HoshiyomiMark className="w-[clamp(9.5rem,15vw,11rem)] shrink-0" />
      <nav
        aria-label={t("modes.navigation")}
        className="min-w-0 flex-1 overflow-x-auto overscroll-x-contain rounded-xl bg-muted/55 p-1 [scrollbar-width:none]"
      >
        <div className="grid min-w-max grid-cols-5 gap-1">
          {MODES.map(({ id, icon: Icon }) => (
            <button
              key={id}
              id={`mode-${id}`}
              type="button"
              aria-label={t(`modes.${id}`)}
              aria-current={mode === id ? "page" : undefined}
              title={t(`modes.${id}`)}
              className={cn(
                "flex min-h-10 min-w-10 items-center justify-center gap-2 rounded-lg px-2.5 text-xs font-medium transition-[background-color,box-shadow,color,transform] duration-[var(--duration-fast)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring active:scale-[0.96] min-[1040px]:px-3",
                mode === id
                  ? "bg-background text-foreground shadow-[var(--shadow-surface)]"
                  : "text-muted-foreground hover:bg-background/60 hover:text-foreground",
              )}
              onClick={() => setMode(id)}
              onKeyDown={(event) => {
                if (event.key === "ArrowRight") {
                  event.preventDefault();
                  moveFrom(id, 1);
                } else if (event.key === "ArrowLeft") {
                  event.preventDefault();
                  moveFrom(id, -1);
                }
              }}
            >
              <Icon className="size-4 shrink-0" aria-hidden="true" />
              <span className="hidden min-[920px]:inline">{t(`modes.${id}`)}</span>
            </button>
          ))}
        </div>
      </nav>
      <Button
        id="settings-trigger"
        size="icon-sm"
        variant="ghost"
        className="ml-auto size-10 shrink-0"
        onClick={onOpenSettings}
        title={t("settings.title")}
        aria-label={t("settings.title")}
      >
        <Settings className="h-4 w-4" />
      </Button>
    </header>
  );
}
