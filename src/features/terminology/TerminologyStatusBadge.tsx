import { cn } from "@/lib/utils";
import type { TerminologyReviewStatus } from "@/lib/types";

const STATUS = {
  proposed: { glyph: "◇", className: "bg-star/12 text-star" },
  approved: { glyph: "✓", className: "bg-primary/10 text-primary" },
  locked: { glyph: "◆", className: "bg-seal/12 text-seal" },
} as const;

export function TerminologyStatusBadge({
  status,
  label,
}: {
  status: TerminologyReviewStatus;
  label: string;
}) {
  const meta = STATUS[status];
  return (
    <span
      className={cn(
        "inline-flex max-w-full items-center gap-1 rounded-full px-2 py-1 text-[10px] font-semibold",
        meta.className,
      )}
    >
      <span aria-hidden="true">{meta.glyph}</span>
      <span className="truncate">{label}</span>
    </span>
  );
}
