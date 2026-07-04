import { useEffect, useState } from "react";
import { Search, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useProjectStore } from "@/stores/project";
import { useSearchStore } from "@/stores/search";
import type { SearchScope } from "@/lib/types";

/**
 * Project-wide search bar (FILES panel) — searches segments across ALL files
 * of the active project, unlike the grid's per-file filter. Commits on Enter
 * (min 2 chars); results replace the grid in the centre panel.
 */
export function ProjectSearchBar() {
  const { t } = useTranslation();
  const activeProjectId = useProjectStore((s) => s.activeProjectId);
  const scope = useSearchStore((s) => s.scope);
  const committedQuery = useSearchStore((s) => s.query);
  const runSearch = useSearchStore((s) => s.runSearch);
  const clear = useSearchStore((s) => s.clear);

  // Local draft — the store only sees committed (Enter) queries. The
  // component is remounted per project (key={activeProjectId} in App.tsx),
  // which resets the draft; clearing the store on mount drops any search
  // state left over from the previous project.
  const [draft, setDraft] = useState("");
  useEffect(() => {
    clear();
  }, [clear]);

  const commit = (query: string, nextScope: SearchScope) => {
    if (!activeProjectId) return;
    if (query.trim().length >= 2) {
      void runSearch(activeProjectId, query, nextScope);
    } else if (query.trim() === "") {
      clear();
    }
  };

  const handleScopeChange = (v: string) => {
    const nextScope = v as SearchScope;
    useSearchStore.setState({ scope: nextScope });
    // Scope change with a committed query → re-run immediately.
    if (committedQuery.length >= 2) commit(committedQuery, nextScope);
  };

  return (
    <div className="shrink-0 border-b px-2 py-1.5 flex flex-col gap-1.5">
      <div className="relative">
        <Search className="absolute left-2 top-1/2 h-3 w-3 -translate-y-1/2 text-muted-foreground" />
        <Input
          className="h-7 text-xs pl-7 pr-7"
          placeholder={t("projectSearch.placeholder")}
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") commit(draft, scope);
          }}
        />
        {draft !== "" && (
          <button
            type="button"
            title={t("projectSearch.clear")}
            className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
            onClick={() => {
              setDraft("");
              clear();
            }}
          >
            <X className="h-3 w-3" />
          </button>
        )}
      </div>

      <Select value={scope} onValueChange={handleScopeChange}>
        <SelectTrigger className="h-7 text-xs">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="both">{t("projectSearch.scopeBoth")}</SelectItem>
          <SelectItem value="source">
            {t("projectSearch.scopeSource")}
          </SelectItem>
          <SelectItem value="target">
            {t("projectSearch.scopeTarget")}
          </SelectItem>
        </SelectContent>
      </Select>
    </div>
  );
}
