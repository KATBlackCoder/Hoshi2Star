import type { ReactNode } from "react";
import { ContextBar } from "@/components/shell/ContextBar";
import { ModeNavigation } from "@/components/shell/ModeNavigation";

export function AppShell({
  children,
  onOpenSettings,
}: {
  children: ReactNode;
  onOpenSettings: () => void;
}) {
  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background text-foreground antialiased">
      <a
        href="#workspace-main"
        className="fixed left-3 top-3 z-[100] -translate-y-20 rounded-lg bg-primary px-3 py-2 text-sm font-medium text-primary-foreground shadow-lg transition-transform focus:translate-y-0"
      >
        Aller au contenu principal
      </a>
      <ModeNavigation onOpenSettings={onOpenSettings} />
      <ContextBar />
      <main
        id="workspace-main"
        tabIndex={-1}
        className="min-h-0 min-w-0 flex-1 overflow-hidden"
      >
        {children}
      </main>
    </div>
  );
}
