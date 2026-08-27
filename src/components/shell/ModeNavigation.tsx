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
  }

  return (
    <header className="flex min-h-14 shrink-0 items-center gap-4 border-b bg-background/85 px-3 backdrop-blur-md">
      <div className="flex min-w-36 items-baseline gap-1.5 select-none">
        <span className="text-star drop-shadow-[0_0_8px_var(--star)]">★</span>
        <span className="text-sm font-semibold tracking-tight">Hoshi2Star</span>
      </div>
      <nav
        aria-label={t("modes.navigation")}
        className="flex min-h-10 items-center gap-1 rounded-xl bg-muted/60 p-1"
      >
        {MODES.map(({ id, icon: Icon }) => (
          <button
            key={id}
            type="button"
            aria-current={mode === id ? "page" : undefined}
            className={cn(
              "flex min-h-10 items-center gap-2 rounded-lg px-3 text-xs font-medium transition-[background-color,box-shadow,color,transform] duration-[var(--duration-fast)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring active:scale-[0.96]",
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
            <Icon className="h-4 w-4" />
            {t(`modes.${id}`)}
          </button>
        ))}
      </nav>
      <Button
        size="icon-sm"
        variant="ghost"
        className="ml-auto"
        onClick={onOpenSettings}
        title={t("settings.title")}
      >
        <Settings className="h-4 w-4" />
      </Button>
    </header>
  );
}
