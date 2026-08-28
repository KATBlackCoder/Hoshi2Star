import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

export function WorkspaceState({
  icon,
  eyebrow,
  title,
  description,
  action,
  tone = "neutral",
  className,
}: {
  icon?: ReactNode;
  eyebrow?: string;
  title: string;
  description?: string;
  action?: ReactNode;
  tone?: "neutral" | "error";
  className?: string;
}) {
  return (
    <section
      className={cn(
        "grid min-h-44 min-w-0 place-items-center rounded-2xl bg-card/82 p-6 text-center shadow-[var(--shadow-surface)]",
        tone === "error" && "shrine-edge",
        className,
      )}
      role={tone === "error" ? "alert" : "status"}
    >
      <div className="min-w-0 max-w-xl">
        {icon && (
          <span className="mx-auto mb-3 grid size-10 place-items-center rounded-xl bg-muted text-muted-foreground">
            {icon}
          </span>
        )}
        {eyebrow && (
          <p className="mb-1 text-[10px] font-semibold uppercase tracking-[0.16em] text-star">
            {eyebrow}
          </p>
        )}
        <h2 className="text-safe font-semibold">{title}</h2>
        {description && (
          <p className="text-safe mt-1 text-sm leading-6 text-muted-foreground">
            {description}
          </p>
        )}
        {action && <div className="mt-4 flex justify-center">{action}</div>}
      </div>
    </section>
  );
}
