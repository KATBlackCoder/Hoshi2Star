import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  PROVIDER_PRESETS,
  RESOURCE_PROFILES,
  type ProviderId,
} from "@/lib/constants";
import type { ResourceProfile } from "@/lib/types";
import type { AppSettings } from "@/stores/settings";

interface ProviderSettingsProps {
  draft: AppSettings;
  models: string[];
  modelsLoading: boolean;
  modelsError: string | null;
  onChange: (patch: Partial<AppSettings>) => void;
  onProviderChange: (providerId: ProviderId) => void;
  onTest: () => void;
  onUrlBlur: () => void;
}

export function ProviderSettings({
  draft,
  models,
  modelsLoading,
  modelsError,
  onChange,
  onProviderChange,
  onTest,
  onUrlBlur,
}: ProviderSettingsProps) {
  const { t } = useTranslation();
  const preset = PROVIDER_PRESETS.find(
    (candidate) => candidate.id === draft.providerId,
  );

  return (
    <section aria-labelledby="settings-provider-heading" className="space-y-5">
      <div>
        <h3 id="settings-provider-heading" className="text-sm font-semibold">
          {t("settings.llm.section")}
        </h3>
        <p className="mt-1 text-xs leading-5 text-muted-foreground">
          {t("settings.llm.description")}
        </p>
      </div>

      <div className="space-y-1.5">
        <label htmlFor="settings-provider" className="text-xs font-medium">
          {t("settings.llm.providerLabel")}
        </label>
        <Select value={draft.providerId} onValueChange={onProviderChange}>
          <SelectTrigger
            id="settings-provider"
            className="min-h-10 w-full text-xs"
          >
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectGroup>
              {PROVIDER_PRESETS.map((provider) => (
                <SelectItem key={provider.id} value={provider.id}>
                  {provider.label}
                </SelectItem>
              ))}
            </SelectGroup>
          </SelectContent>
        </Select>
      </div>

      <div className="flex flex-col gap-1.5">
        <label htmlFor="settings-resource-profile" className="text-xs font-medium">
          {t("settings.llm.resourceProfileLabel")}
        </label>
        <Select
          value={draft.resourceProfile}
          onValueChange={(value) => {
            const resourceProfile = value as ResourceProfile;
            const profile = RESOURCE_PROFILES.find(
              (candidate) => candidate.id === resourceProfile,
            );
            onChange({
              resourceProfile,
              batchSize: profile?.batchSize ?? draft.batchSize,
            });
          }}
        >
          <SelectTrigger
            id="settings-resource-profile"
            className="min-h-10 w-full text-xs"
          >
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectGroup>
              {RESOURCE_PROFILES.map((profile) => (
                <SelectItem key={profile.id} value={profile.id}>
                  {t(`settings.llm.resourceProfiles.${profile.id}`)}
                </SelectItem>
              ))}
            </SelectGroup>
          </SelectContent>
        </Select>
        <p className="text-[11px] leading-4 text-muted-foreground">
          {t(`settings.llm.resourceProfileHints.${draft.resourceProfile}`)}
        </p>
      </div>

      <div className="space-y-1.5">
        <label htmlFor="settings-provider-url" className="text-xs font-medium">
          {t("settings.llm.urlLabel")}
        </label>
        <div className="flex gap-2">
          <Input
            id="settings-provider-url"
            className="min-h-10 text-xs"
            value={draft.ollamaUrl}
            onChange={(event) => onChange({ ollamaUrl: event.target.value })}
            onBlur={onUrlBlur}
          />
          <Button
            type="button"
            variant="outline"
            className="min-h-10 shrink-0 text-xs"
            onClick={onTest}
            disabled={modelsLoading}
          >
            {t("settings.llm.testButton")}
          </Button>
        </div>
      </div>

      {preset?.requiresApiKey && (
        <div className="space-y-1.5">
          <label htmlFor="settings-api-key" className="text-xs font-medium">
            {t("settings.llm.apiKeyLabel")}
          </label>
          <Input
            id="settings-api-key"
            type="password"
            autoComplete="off"
            className="min-h-10 text-xs"
            value={draft.apiKey}
            onChange={(event) => onChange({ apiKey: event.target.value })}
            placeholder={t("settings.llm.apiKeyPlaceholder")}
          />
          <p className="text-[11px] leading-4 text-muted-foreground">
            {t("settings.llm.apiKeyHint")}
          </p>
        </div>
      )}

      <div className="space-y-1.5">
        <label htmlFor="settings-model" className="text-xs font-medium">
          {t("settings.llm.modelLabel")}
        </label>
        {models.length > 0 ? (
          <Select
            value={draft.ollamaModel}
            onValueChange={(ollamaModel) => onChange({ ollamaModel })}
          >
            <SelectTrigger
              id="settings-model"
              className="min-h-10 w-full text-xs"
            >
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                {models.map((model) => (
                  <SelectItem key={model} value={model} className="text-xs">
                    {model}
                  </SelectItem>
                ))}
              </SelectGroup>
            </SelectContent>
          </Select>
        ) : (
          <Input
            id="settings-model"
            className="min-h-10 text-xs"
            placeholder={
              modelsLoading
                ? t("settings.llm.modelLoading")
                : t("settings.llm.modelManual")
            }
            value={draft.ollamaModel}
            onChange={(event) => onChange({ ollamaModel: event.target.value })}
          />
        )}
        {modelsError && (
          <p role="alert" className="text-[11px] text-destructive">
            {modelsError}
          </p>
        )}
      </div>

      <div className="space-y-1.5">
        <label htmlFor="settings-batch-size" className="text-xs font-medium">
          {t("settings.llm.batchSizeLabel")}
        </label>
        <Input
          id="settings-batch-size"
          type="number"
          min={1}
          max={100}
          className="min-h-10 text-xs"
          value={draft.batchSize}
          onChange={(event) =>
            onChange({ batchSize: Number(event.target.value) })
          }
          onBlur={() =>
            onChange({
              batchSize: Math.min(100, Math.max(1, draft.batchSize || 1)),
            })
          }
        />
        <p className="text-[11px] leading-4 text-muted-foreground">
          {t("settings.llm.batchSizeHint")}
        </p>
      </div>
    </section>
  );
}
