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
      <ModeNavigation onOpenSettings={onOpenSettings} />
      <ContextBar />
      <main className="min-h-0 flex-1 overflow-hidden">{children}</main>
    </div>
  );
}
