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

      {isFiltered && (
        <span className="shrink-0 text-xs text-muted-foreground tabular-nums">
          {shownCount} / {totalCount}
        </span>
      )}
    </div>
  );
}
