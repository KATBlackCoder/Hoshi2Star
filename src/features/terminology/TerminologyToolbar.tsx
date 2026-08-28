import { BookOpenCheck, Languages, Plus, Search } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { PartOfSpeech, TerminologyEntryStatus } from "@/lib/types";
import { useTranslation } from "react-i18next";

export function TerminologyToolbar(props: {
  search: string;
  onSearch: (value: string) => void;
  partOfSpeech: PartOfSpeech | "all";
  onPartOfSpeech: (value: PartOfSpeech | "all") => void;
  semanticType: string;
  onSemanticType: (value: string) => void;
  status: TerminologyEntryStatus | "all";
  onStatus: (value: TerminologyEntryStatus | "all") => void;
  targetLanguage: string;
  onTargetLanguage: (value: string) => void;
  projectAvailable: boolean;
  scope: "project" | "global";
  onScope: (value: "project" | "global") => void;
  canScan: boolean;
  scanDisabledReason?: string;
  scanning: boolean;
  onScan: () => void;
  onCreate: () => void;
  onTranslate: () => void;
}) {
  const { t } = useTranslation();
  return (
    <div className="flex min-w-0 shrink-0 flex-wrap items-end gap-2 rounded-2xl bg-card/80 p-3 shadow-[var(--shadow-surface)]">
      <label className="min-w-52 flex-1 text-xs text-muted-foreground">
        {t("terminology.search")}
        <span className="relative mt-1 block">
          <Search
            className="pointer-events-none absolute left-2.5 top-1/2 size-4 -translate-y-1/2"
            aria-hidden="true"
          />
          <Input
            value={props.search}
            onChange={(event) => props.onSearch(event.target.value)}
            className="pl-8"
            placeholder={t("terminology.searchPlaceholder")}
          />
        </span>
      </label>
      <Filter
        label={t("terminology.partOfSpeech")}
        value={props.partOfSpeech}
        onChange={(value) =>
          props.onPartOfSpeech(value as PartOfSpeech | "all")
        }
        options={[
          ["all", t("terminology.all")],
          ["noun", "Nom"],
          ["proper_noun", "Nom propre"],
          ["verb", "Verbe"],
          ["adjective", "Adjectif"],
          ["adverb", "Adverbe"],
          ["expression", "Expression"],
        ]}
      />
      <label className="shrink-0 text-xs text-muted-foreground">
        {t("terminology.semanticType")}
        <Input
          className="mt-1 w-32"
          value={props.semanticType}
          onChange={(event) => props.onSemanticType(event.target.value)}
          placeholder="Tous"
        />
      </label>
      <Filter
        label={t("terminology.status")}
        value={props.status}
        onChange={(value) =>
          props.onStatus(value as TerminologyEntryStatus | "all")
        }
        options={[
          ["active", t("terminology.active")],
          ["ignored", t("terminology.ignored")],
          ["archived", t("terminology.archived")],
          ["all", t("terminology.all")],
        ]}
      />
      <Filter
        label={t("terminology.target")}
        value={props.targetLanguage}
        onChange={props.onTargetLanguage}
        options={[
          ["en", "English"],
          ["fr", "Français"],
        ]}
      />
      <Filter
        label={t("terminology.scope")}
        value={props.projectAvailable ? props.scope : "global"}
        disabled={!props.projectAvailable}
        onChange={(value) => props.onScope(value as "project" | "global")}
        options={[
          ["project", t("terminology.project")],
          ["global", t("terminology.global")],
        ]}
      />
      <div className="ml-auto flex min-w-0 flex-wrap justify-end gap-2">
        <Button variant="outline" onClick={props.onCreate}>
          <Plus /> {t("terminology.add")}
        </Button>
        <Button variant="outline" onClick={props.onTranslate}>
          <Languages /> {t("terminology.translateTerms")}
        </Button>
        <Button
          onClick={props.onScan}
          disabled={!props.canScan || props.scanning}
          title={props.scanDisabledReason}
        >
          <BookOpenCheck />{" "}
          {props.scanning ? t("terminology.scanning") : t("terminology.scan")}
        </Button>
      </div>
    </div>
  );
}

function Filter({
  label,
  value,
  options,
  onChange,
  disabled,
}: {
  label: string;
  value: string;
  options: string[][];
  onChange: (value: string) => void;
  disabled?: boolean;
}) {
  return (
    <label className="shrink-0 text-xs text-muted-foreground">
      {label}
      <select
        className="mt-1 block h-10 rounded-lg border bg-background px-2 text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        value={value}
        onChange={(event) => onChange(event.target.value)}
        disabled={disabled}
      >
        {options.map(([option, text]) => (
          <option value={option} key={option}>
            {text}
          </option>
        ))}
      </select>
    </label>
  );
}
