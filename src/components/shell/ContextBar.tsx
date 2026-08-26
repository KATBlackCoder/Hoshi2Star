import { Cpu, Languages } from "lucide-react";
import { useProjectStore } from "@/stores/project";
import { useProviderConfig } from "@/stores/llm";
import { PROVIDER_PRESETS } from "@/lib/constants";
import { engineLabel } from "@/lib/format";

export function ContextBar() {
  const project = useProjectStore((state) =>
    state.projects.find((candidate) => candidate.id === state.activeProjectId),
  );
  const provider = useProviderConfig();
  if (!project) return null;

  const providerLabel =
    PROVIDER_PRESETS.find((preset) => preset.id === provider.providerId)?.label ??
    "Compatible OpenAI";

  return (
    <div className="flex min-h-11 shrink-0 items-center gap-3 border-b bg-card/45 px-4 text-xs">
      <strong className="min-w-0 truncate text-sm font-medium">{project.name}</strong>
      <span className="rounded-full bg-muted px-2 py-1 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
        {engineLabel(project.engine)}
      </span>
      <span className="flex items-center gap-1.5 rounded-full bg-primary/10 px-2 py-1 font-mono text-[10px] uppercase tracking-wide text-primary">
        <Languages className="h-3 w-3" />
        {project.sourceLang} → {project.targetLang}
      </span>
      <span className="ml-auto flex items-center gap-1.5 text-muted-foreground">
        <Cpu className="h-3.5 w-3.5" />
        {providerLabel}
        {provider.model && <span className="max-w-48 truncate">· {provider.model}</span>}
      </span>
    </div>
  );
}
