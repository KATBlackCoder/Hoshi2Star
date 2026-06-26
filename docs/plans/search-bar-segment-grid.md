# Search Bar + Column Filters for SegmentGrid — Implementation Plan

**Goal:** Add a text search input (source + target) and a status/QA dropdown filter above the segment grid, with a live segment count badge showing filtered vs total.

**Architecture:** All filtering stays client-side in `filteredSegments` useMemo — segments are already loaded in memory (up to 5000 via `get_segments`). Add `searchQuery: string` state alongside the existing `qaFilter`. The `filteredSegments` memo applies both filters in sequence. Extract the toolbar into a focused `SegmentSearchBar` component. Zero Rust changes.

**Tech Stack:** React 19, TanStack Table v8 (no new imports needed), shadcn Input, existing Select + i18n (react-i18next).

---

## File map

| File | Action |
|---|---|
| `src/components/editor/SegmentSearchBar.tsx` | **Create** — Input + Select + count badge |
| `src/components/editor/SegmentGrid.tsx` | **Modify** — add `searchQuery` state, update `filteredSegments` memo, replace inline toolbar with `<SegmentSearchBar>` |
| `src/locales/en.json` | **Modify** — add `search.*` keys |
| `src/locales/fr.json` | **Modify** — add `search.*` keys |

---

### Task 1: Add i18n keys

**Files:**
- Modify: `src/locales/en.json`
- Modify: `src/locales/fr.json`

- [ ] **Step 1: Add keys to en.json**

Inside the `"segmentGrid"` object, add after `"noModelConfigured"`:

```json
"searchPlaceholder": "Search source or target…",
"searchClear": "Clear search",
"filterLabel": "Filter",
"footerFilteredSearch": "{{shown}} / {{total}} segments"
```

- [ ] **Step 2: Add keys to fr.json**

Inside the `"segmentGrid"` object, add after `"noModelConfigured"`:

```json
"searchPlaceholder": "Rechercher dans source ou cible…",
"searchClear": "Effacer la recherche",
"filterLabel": "Filtre",
"footerFilteredSearch": "{{shown}} / {{total}} segments"
```

- [ ] **Step 3: Typecheck**

```bash
pnpm typecheck
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/locales/en.json src/locales/fr.json
git commit -m "feat(search): add i18n keys for segment search bar"
```

---

### Task 2: Create SegmentSearchBar component

**Files:**
- Create: `src/components/editor/SegmentSearchBar.tsx`

This component receives all filter state as props and owns no state itself — `SegmentGrid` is the source of truth.

- [ ] **Step 1: Create the component**

```tsx
import { X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

export type QaFilter =
  | "all"
  | "errors"
  | "critical"
  | "untranslated"
  | "needs_review";

interface SegmentSearchBarProps {
  searchQuery: string;
  onSearchChange: (value: string) => void;
  qaFilter: QaFilter;
  onQaFilterChange: (value: QaFilter) => void;
  shownCount: number;
  totalCount: number;
}

export function SegmentSearchBar({
  searchQuery,
  onSearchChange,
  qaFilter,
  onQaFilterChange,
  shownCount,
  totalCount,
}: SegmentSearchBarProps) {
  const { t } = useTranslation();
  const isFiltered = searchQuery !== "" || qaFilter !== "all";

  return (
    <div className="shrink-0 border-b px-3 py-1.5 flex items-center gap-2">
      {/* Text search */}
      <div className="relative flex-1 min-w-0">
        <Input
          className="h-7 text-xs pr-7"
          placeholder={t("segmentGrid.searchPlaceholder")}
          value={searchQuery}
          onChange={(e) => onSearchChange(e.target.value)}
        />
        {searchQuery !== "" && (
          <button
            type="button"
            title={t("segmentGrid.searchClear")}
            className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
            onClick={() => onSearchChange("")}
          >
            <X className="h-3 w-3" />
          </button>
        )}
      </div>

      {/* Status / QA filter */}
      <Select
        value={qaFilter}
        onValueChange={(v) => onQaFilterChange(v as QaFilter)}
      >
        <SelectTrigger className="h-7 w-44 text-xs shrink-0">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="all">{t("segmentGrid.filterAll")}</SelectItem>
          <SelectItem value="untranslated">
            {t("segmentGrid.filterUntranslated")}
          </SelectItem>
          <SelectItem value="needs_review">
            {t("segmentGrid.filterNeedsReview")}
          </SelectItem>
          <SelectItem value="errors">
            {t("segmentGrid.filterQaErrors")}
          </SelectItem>
          <SelectItem value="critical">
            {t("segmentGrid.filterQaCritical")}
          </SelectItem>
        </SelectContent>
      </Select>

      {/* Count badge — only when filtered */}
      {isFiltered && (
        <span className="shrink-0 text-xs text-muted-foreground tabular-nums">
          {shownCount} / {totalCount}
        </span>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Typecheck**

```bash
pnpm typecheck
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/components/editor/SegmentSearchBar.tsx
git commit -m "feat(search): add SegmentSearchBar component"
```

---

### Task 3: Wire search into SegmentGrid

**Files:**
- Modify: `src/components/editor/SegmentGrid.tsx`

Four changes:
1. Import `SegmentSearchBar` + `QaFilter` type
2. Add `searchQuery` state, change `qaFilter` type to `QaFilter`
3. Update `filteredSegments` memo to apply text search after QA filter
4. Replace the inline toolbar Select with `<SegmentSearchBar>`

- [ ] **Step 1: Update imports at the top of SegmentGrid.tsx**

Replace:
```tsx
import { Play, RefreshCw } from "lucide-react";
```
With:
```tsx
import { Play, RefreshCw } from "lucide-react";
import { SegmentSearchBar, type QaFilter } from "@/components/editor/SegmentSearchBar";
```

- [ ] **Step 2: Replace qaFilter state type and add searchQuery state**

Replace:
```tsx
const [qaFilter, setQaFilter] = useState<
  "all" | "errors" | "critical" | "untranslated" | "needs_review"
>("all");
```
With:
```tsx
const [qaFilter, setQaFilter] = useState<QaFilter>("all");
const [searchQuery, setSearchQuery] = useState("");
```

- [ ] **Step 3: Reset searchQuery when switching files**

Replace:
```tsx
  useEffect(() => {
    setQaFilter("all");
    setRowSelection({});
  }, [activeFileId]);
```
With:
```tsx
  useEffect(() => {
    setQaFilter("all");
    setSearchQuery("");
    setRowSelection({});
  }, [activeFileId]);
```

- [ ] **Step 4: Update filteredSegments memo to apply text search**

Replace:
```tsx
  const filteredSegments = useMemo(() => {
    switch (qaFilter) {
      case "errors":
        return segments.filter(
          (s) =>
            s.qaScore !== null && s.qaScore !== undefined && s.qaScore < 100,
        );
      case "critical":
        return segments.filter(
          (s) =>
            s.qaScore !== null && s.qaScore !== undefined && s.qaScore < 70,
        );
      case "untranslated":
        return segments.filter((s) => s.status === "untranslated");
      case "needs_review":
        return segments.filter((s) => s.status === "needs_review");
      default:
        return segments;
    }
  }, [segments, qaFilter]);
```
With:
```tsx
  const filteredSegments = useMemo(() => {
    let result = segments;

    switch (qaFilter) {
      case "errors":
        result = result.filter(
          (s) => s.qaScore !== null && s.qaScore !== undefined && s.qaScore < 100,
        );
        break;
      case "critical":
        result = result.filter(
          (s) => s.qaScore !== null && s.qaScore !== undefined && s.qaScore < 70,
        );
        break;
      case "untranslated":
        result = result.filter((s) => s.status === "untranslated");
        break;
      case "needs_review":
        result = result.filter((s) => s.status === "needs_review");
        break;
    }

    if (searchQuery.trim() !== "") {
      const q = searchQuery.toLowerCase();
      result = result.filter(
        (s) =>
          s.sourceText.toLowerCase().includes(q) ||
          s.targetText.toLowerCase().includes(q),
      );
    }

    return result;
  }, [segments, qaFilter, searchQuery]);
```

- [ ] **Step 5: Replace the inline toolbar with SegmentSearchBar**

Replace the entire `{/* Toolbar: QA filter + batch translate */}` div:
```tsx
      {/* Toolbar: QA filter + batch translate */}
      <div className="shrink-0 border-b px-3 py-1.5 flex items-center gap-2">
        <Select
          value={qaFilter}
          onValueChange={(v) =>
            setQaFilter(
              v as
                | "all"
                | "errors"
                | "critical"
                | "untranslated"
                | "needs_review",
            )
          }
        >
          <SelectTrigger className="h-7 w-48 text-xs">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">{t("segmentGrid.filterAll")}</SelectItem>
            <SelectItem value="errors">
              {t("segmentGrid.filterQaErrors")}
            </SelectItem>
            <SelectItem value="critical">
              {t("segmentGrid.filterQaCritical")}
            </SelectItem>
            <SelectItem value="untranslated">
              {t("segmentGrid.filterUntranslated")}
            </SelectItem>
            <SelectItem value="needs_review">
              {t("segmentGrid.filterNeedsReview")}
            </SelectItem>
          </SelectContent>
        </Select>

        {needsReviewIds.length > 0 && (
          ...
        )}

        {selectedIds.length >= 2 && (
          ...
        )}
      </div>
```

With:
```tsx
      {/* Search bar + QA filter */}
      <SegmentSearchBar
        searchQuery={searchQuery}
        onSearchChange={setSearchQuery}
        qaFilter={qaFilter}
        onQaFilterChange={setQaFilter}
        shownCount={filteredSegments.length}
        totalCount={segments.length}
      />

      {/* Batch action toolbar */}
      {(needsReviewIds.length > 0 || selectedIds.length >= 2) && (
        <div className="shrink-0 border-b px-3 py-1.5 flex items-center gap-2">
          {needsReviewIds.length > 0 && (
            <Button
              size="sm"
              variant="outline"
              className="h-7 gap-1.5 text-xs text-yellow-400 border-yellow-400/40 hover:bg-yellow-400/10"
              disabled={isTranslating}
              onClick={() => {
                if (!providerConfig.model.trim()) {
                  toast.error(t("segmentGrid.noModelConfigured"));
                  return;
                }
                void startTranslation(needsReviewIds, undefined);
              }}
            >
              <RefreshCw className="h-3 w-3" />
              {t("segmentGrid.retranslateNeedsReview", {
                count: needsReviewIds.length,
              })}
            </Button>
          )}

          {selectedIds.length >= 2 && (
            <Button
              size="sm"
              variant="outline"
              className="h-7 gap-1.5 text-xs"
              disabled={isTranslating}
              onClick={handleTranslateSelected}
            >
              <Play className="h-3 w-3" />
              {t("segmentGrid.translateSelected", { count: selectedIds.length })}
            </Button>
          )}
        </div>
      )}
```

- [ ] **Step 6: Remove unused Select imports**

At the top of SegmentGrid.tsx, remove the Select imports that are no longer used directly:
```tsx
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
```

- [ ] **Step 7: Typecheck**

```bash
pnpm typecheck
```

Expected: no errors.

- [ ] **Step 8: Commit**

```bash
git add src/components/editor/SegmentGrid.tsx
git commit -m "feat(search): wire SegmentSearchBar into SegmentGrid"
```

---

### Task 4: Manual verification

- [ ] **Step 1: Start dev server**

```bash
pnpm tauri dev
```

- [ ] **Step 2: Test text search**

1. Open a project with a translated file
2. Type a Japanese character in the search input — only matching source rows appear
3. Type an English word — only matching target rows appear
4. Clear with the ✕ button — all rows return
5. Combine: type a word + set filter to "Untranslated" — both filters apply simultaneously

- [ ] **Step 3: Test count badge**

1. No filter active → badge hidden
2. Type anything → badge shows "N / total"
3. Clear search, change dropdown → badge shows "N / total"
4. Clear everything → badge hidden again

- [ ] **Step 4: Test file switch**

Switch to a different file — search input clears, filter resets to "All segments".

- [ ] **Step 5: Test batch toolbar**

The "Retry N failed" and "Translate N lines" buttons still appear when needed.

- [ ] **Step 6: Final commit**

```bash
git add -A
git commit -m "feat(search): segment search bar + column filter complete"
```
