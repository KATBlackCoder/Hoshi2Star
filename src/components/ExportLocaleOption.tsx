import { useTranslation } from "react-i18next";
import { Checkbox } from "@/components/ui/checkbox";
import type { LocaleOption } from "@/lib/types";

// ---------------------------------------------------------------------------
// ExportLocaleOption — optional language-setting switch in the export confirm
// ---------------------------------------------------------------------------

interface ExportLocaleOptionProps {
  option: LocaleOption;
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
}

export function ExportLocaleOption({
  option,
  checked,
  onCheckedChange,
}: ExportLocaleOptionProps) {
  const { t } = useTranslation();
  return (
    <div className="rounded-lg border p-3 text-left">
      <div className="flex items-start gap-3">
        <Checkbox
          id="export-switch-locale"
          className="mt-0.5"
          checked={checked}
          onCheckedChange={(value) => onCheckedChange(value === true)}
          aria-describedby="export-switch-locale-details"
        />
        <div className="min-w-0 flex-1">
          <label htmlFor="export-switch-locale" className="text-sm font-medium">
            {t("exportLocale.label", {
              current: option.current,
              target: option.target,
            })}
          </label>
          <p className="mt-0.5 text-xs text-muted-foreground">
            {t("exportLocale.hint")}
          </p>
          <ul
            id="export-switch-locale-details"
            className="mt-2 list-disc space-y-1 pl-4 text-xs"
          >
            {option.consequences.map((line) => (
              <li key={line}>{line}</li>
            ))}
          </ul>
          {option.uses.length > 0 && (
            <details className="mt-2 text-xs">
              <summary className="cursor-pointer text-primary">
                {t("exportLocale.codeLines", { count: option.uses.length })}
              </summary>
              <ul className="mt-1 max-h-40 space-y-1 overflow-y-auto font-mono break-all">
                {option.uses.map((use) => (
                  <li key={use.location}>
                    <span className="text-muted-foreground">
                      {use.location}
                    </span>{" "}
                    {use.code}
                  </li>
                ))}
              </ul>
            </details>
          )}
        </div>
      </div>
    </div>
  );
}
