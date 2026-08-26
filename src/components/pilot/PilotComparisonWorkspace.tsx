import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import {
  ArrowLeft,
  Check,
  CircleAlert,
  FlaskConical,
  Gauge,
  Layers3,
  Loader2,
  RefreshCcw,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { cn } from "@/lib/utils";
import type {
  PilotComparison,
  PilotQualityFlag,
  PilotSampleCategory,
  PilotVariantResult,
  PilotVariantSummary,
} from "@/lib/types";
import { useProviderConfig } from "@/stores/llm";
import {
  PILOT_PHASES,
  type PilotCategoryFilter,
  type PilotReviewDecision,
  usePilotStore,
} from "@/stores/pilot";
import { useActiveProject } from "@/stores/project";

const CATEGORIES: PilotSampleCategory[] = [
  "dialogue",
  "choice_branch",
  "database",
  "names_ui",
  "placeholder_multiline",
  "uncertain",
];

const DECISIONS: PilotReviewDecision[] = [
  "baseline",
  "contextual",
  "equivalent",
  "review",
];

export function PilotComparisonWorkspace() {
  const { t } = useTranslation();
  const project = useActiveProject();
  const providerConfig = useProviderConfig();
  const isRunning = usePilotStore((state) => state.isRunning);
  const sampleSize = usePilotStore((state) => state.sampleSize);
  const report = usePilotStore((state) => state.report);
  const error = usePilotStore((state) => state.error);
  const categoryFilter = usePilotStore((state) => state.categoryFilter);
  const decisions = usePilotStore((state) => state.decisions);
  const closeWorkspace = usePilotStore((state) => state.closeWorkspace);
  const setSampleSize = usePilotStore((state) => state.setSampleSize);
  const setCategoryFilter = usePilotStore((state) => state.setCategoryFilter);
  const setDecision = usePilotStore((state) => state.setDecision);
  const clearReport = usePilotStore((state) => state.clearReport);
  const runPilot = usePilotStore((state) => state.runPilot);

  const visibleComparisons = useMemo(() => {
    if (!report) return [];
    if (categoryFilter === "all") return report.comparisons;
    return report.comparisons.filter(
      (comparison) => comparison.segment.category === categoryFilter,
    );
  }, [categoryFilter, report]);

  if (!project || project.engine !== "mv_mz") {
    return (
      <div className="grid h-full place-items-center p-8">
        <div className="max-w-md rounded-2xl border bg-card p-6 text-center shadow-sm">
          <CircleAlert className="mx-auto size-6 text-muted-foreground" />
          <p className="mt-3 font-medium">{t("pilot.mvMzOnly")}</p>
          <Button className="mt-5" variant="outline" onClick={closeWorkspace}>
            {t("pilot.back")}
          </Button>
        </div>
      </div>
    );
  }

  return (
    <section className="flex min-h-0 flex-1 flex-col overflow-hidden bg-muted/20">
      <header className="shrink-0 border-b bg-background/95 px-5 py-4 backdrop-blur-sm">
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div className="flex min-w-0 items-start gap-3">
            <Button
              aria-label={t("pilot.back")}
              className="mt-0.5 size-8 shrink-0"
              disabled={isRunning}
              onClick={closeWorkspace}
              size="icon"
              variant="ghost"
            >
              <ArrowLeft />
            </Button>
            <div className="min-w-0">
              <div className="flex flex-wrap items-center gap-2">
                <h1 className="text-lg font-semibold tracking-tight">
                  {t("pilot.title")}
                </h1>
                <Badge variant="secondary">MV / MZ</Badge>
                <Badge variant="outline" className="max-w-56 truncate">
                  {providerConfig.model}
                </Badge>
              </div>
              <p className="mt-1 max-w-3xl text-sm leading-5 text-muted-foreground">
                {t("pilot.subtitle", { project: project.name })}
              </p>
            </div>
          </div>
          {report && !isRunning && (
            <Button
              className="gap-2"
              onClick={clearReport}
              size="sm"
              variant="outline"
            >
              <RefreshCcw />
              {t("pilot.newRun")}
            </Button>
          )}
        </div>
      </header>

      {!report ? (
        <ScrollArea className="min-h-0 flex-1">
          <div className="mx-auto grid w-full max-w-5xl gap-5 p-5 lg:grid-cols-[minmax(0,1fr)_320px]">
            <SetupPanel
              error={error}
              isRunning={isRunning}
              onRun={() => void runPilot(project, providerConfig)}
              sampleSize={sampleSize}
              setSampleSize={setSampleSize}
            />
            <ProgressPanel isRunning={isRunning} />
          </div>
        </ScrollArea>
      ) : (
        <>
          <div className="shrink-0 border-b bg-background px-5 py-4">
            <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
              <SummaryCard
                icon={Gauge}
                label={t("pilot.baseline")}
                summary={report.baseline}
              />
              <SummaryCard
                accent
                icon={Sparkles}
                label={t("pilot.contextual")}
                summary={report.contextual}
              />
              <StatCard
                icon={Layers3}
                label={t("pilot.changed")}
                value={`${report.changedCount} / ${report.comparisons.length}`}
              />
              <StatCard
                icon={ShieldCheck}
                label={t("pilot.isolation")}
                value={
                  report.preparation.isolation.databaseRemoved
                    ? t("pilot.isolated")
                    : t("pilot.isolationFailed")
                }
              />
            </div>
          </div>

          <div className="flex min-h-0 flex-1 flex-col">
            <div className="flex shrink-0 flex-wrap items-center justify-between gap-3 border-b bg-background/80 px-5 py-3">
              <p className="text-sm text-muted-foreground">
                {t("pilot.visible", {
                  visible: visibleComparisons.length,
                  total: report.comparisons.length,
                })}
              </p>
              <Select
                value={categoryFilter}
                onValueChange={(value) =>
                  setCategoryFilter(value as PilotCategoryFilter)
                }
              >
                <SelectTrigger
                  aria-label={t("pilot.filterLabel")}
                  className="w-56"
                  size="sm"
                >
                  <SelectValue />
                </SelectTrigger>
                <SelectContent align="end">
                  <SelectItem value="all">{t("pilot.category.all")}</SelectItem>
                  {CATEGORIES.map((category) => (
                    <SelectItem key={category} value={category}>
                      {t(`pilot.category.${category}`)}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <ScrollArea className="min-h-0 flex-1">
              <div className="mx-auto grid max-w-7xl gap-4 p-5">
                {visibleComparisons.map((comparison, index) => (
                  <ComparisonCard
                    comparison={comparison}
                    decision={decisions[comparison.segment.stableKey]}
                    index={index + 1}
                    key={comparison.segment.stableKey}
                    onDecision={(decision) =>
                      setDecision(comparison.segment.stableKey, decision)
                    }
                  />
                ))}
              </div>
            </ScrollArea>
          </div>
        </>
      )}
    </section>
  );
}

function SetupPanel({
  error,
  isRunning,
  onRun,
  sampleSize,
  setSampleSize,
}: {
  error: string | null;
  isRunning: boolean;
  onRun: () => void;
  sampleSize: number;
  setSampleSize: (value: number) => void;
}) {
  const { t } = useTranslation();
  return (
    <div className="rounded-2xl border bg-card p-6 shadow-sm">
      <div className="flex items-start gap-3">
        <div className="grid size-10 shrink-0 place-items-center rounded-xl bg-primary/10 text-primary">
          <FlaskConical className="size-5" />
        </div>
        <div>
          <h2 className="font-semibold">{t("pilot.setupTitle")}</h2>
          <p className="mt-1 text-sm leading-6 text-muted-foreground">
            {t("pilot.setupDescription")}
          </p>
        </div>
      </div>

      <div className="mt-6 rounded-xl border bg-muted/30 p-4">
        <label className="text-sm font-medium" htmlFor="pilot-sample-size">
          {t("pilot.sampleSize")}
        </label>
        <div className="mt-2 flex items-center gap-3">
          <Input
            className="w-28"
            disabled={isRunning}
            id="pilot-sample-size"
            inputMode="numeric"
            max={50}
            min={4}
            onChange={(event) => setSampleSize(Number(event.target.value))}
            type="number"
            value={sampleSize}
          />
          <p className="text-xs leading-5 text-muted-foreground">
            {t("pilot.sampleHint", { calls: sampleSize * 2 })}
          </p>
        </div>
      </div>

      <div className="mt-5 grid gap-2 text-sm text-muted-foreground sm:grid-cols-3">
        {["sameSample", "automaticQa", "noPersistence"].map((item) => (
          <div className="flex items-center gap-2" key={item}>
            <Check className="size-4 shrink-0 text-emerald-500" />
            <span>{t(`pilot.${item}`)}</span>
          </div>
        ))}
      </div>

      {error && (
        <div
          className="mt-5 rounded-xl border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive"
          role="alert"
        >
          {error}
        </div>
      )}

      <Button
        className="mt-6 w-full gap-2 sm:w-auto"
        disabled={isRunning}
        onClick={onRun}
      >
        {isRunning ? (
          <Loader2 className="motion-safe:animate-spin" />
        ) : (
          <FlaskConical />
        )}
        {isRunning ? t("pilot.running") : t("pilot.start")}
      </Button>
    </div>
  );
}

function ProgressPanel({ isRunning }: { isRunning: boolean }) {
  const { t } = useTranslation();
  const progress = usePilotStore((state) => state.progress);
  const phaseIndex = progress ? PILOT_PHASES.indexOf(progress.phase) : -1;
  const withinPhase =
    progress && progress.total > 0
      ? Math.round((progress.done / progress.total) * 100)
      : 0;

  return (
    <aside className="rounded-2xl border bg-card p-5 shadow-sm">
      <h2 className="text-sm font-semibold">{t("pilot.workflow")}</h2>
      <div className="mt-4 space-y-1">
        {PILOT_PHASES.map((phase, index) => {
          const active = isRunning && index === phaseIndex;
          const complete =
            phaseIndex > index || (!isRunning && phaseIndex === index);
          return (
            <div
              className={cn(
                "flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm",
                active && "bg-primary/8 text-foreground",
                !active && !complete && "text-muted-foreground",
              )}
              key={phase}
            >
              <span
                className={cn(
                  "grid size-6 place-items-center rounded-full border text-[11px] font-medium",
                  complete &&
                    "border-emerald-500/30 bg-emerald-500/10 text-emerald-600",
                  active && "border-primary/30 bg-primary/10 text-primary",
                )}
              >
                {complete ? <Check className="size-3.5" /> : index + 1}
              </span>
              <span className={cn(active && "font-medium")}>
                {t(`pilot.phase.${phase}`)}
              </span>
              {active && (
                <span className="ml-auto text-xs tabular-nums text-primary">
                  {withinPhase}%
                </span>
              )}
            </div>
          );
        })}
      </div>
      <div className="mt-5 h-1.5 overflow-hidden rounded-full bg-muted">
        <div
          className="h-full rounded-full bg-primary transition-[width] duration-300 motion-reduce:transition-none"
          style={{
            width: `${Math.max(0, ((phaseIndex + withinPhase / 100) / PILOT_PHASES.length) * 100)}%`,
          }}
        />
      </div>
      <p className="mt-4 text-xs leading-5 text-muted-foreground">
        {t("pilot.resourceNote")}
      </p>
    </aside>
  );
}

function SummaryCard({
  accent = false,
  icon: Icon,
  label,
  summary,
}: {
  accent?: boolean;
  icon: typeof Gauge;
  label: string;
  summary: PilotVariantSummary;
}) {
  const { t, i18n } = useTranslation();
  return (
    <div
      className={cn(
        "rounded-xl border bg-card p-4 shadow-sm",
        accent && "border-primary/30 ring-1 ring-primary/10",
      )}
    >
      <div className="flex items-center justify-between gap-2">
        <p className="text-sm font-medium">{label}</p>
        <Icon
          className={cn(
            "size-4 text-muted-foreground",
            accent && "text-primary",
          )}
        />
      </div>
      <div className="mt-3 flex items-end justify-between gap-3">
        <div>
          <p className="text-2xl font-semibold tabular-nums">
            {summary.averageQaScore.toFixed(0)}
          </p>
          <p className="text-xs text-muted-foreground">
            {t("pilot.qaAverage")}
          </p>
        </div>
        <div className="text-right text-xs leading-5 text-muted-foreground">
          <p>
            {formatTokenCount(summary.metrics.totalTokens, i18n.language)} ·{" "}
            {summary.metrics.requestCount} {t("pilot.requests")}
          </p>
          <p>{formatDuration(summary.metrics.durationMs)}</p>
        </div>
      </div>
    </div>
  );
}

function StatCard({
  icon: Icon,
  label,
  value,
}: {
  icon: typeof Gauge;
  label: string;
  value: string;
}) {
  return (
    <div className="rounded-xl border bg-card p-4 shadow-sm">
      <div className="flex items-center justify-between gap-2">
        <p className="text-sm text-muted-foreground">{label}</p>
        <Icon className="size-4 text-muted-foreground" />
      </div>
      <p className="mt-3 text-xl font-semibold tabular-nums">{value}</p>
    </div>
  );
}

function ComparisonCard({
  comparison,
  decision,
  index,
  onDecision,
}: {
  comparison: PilotComparison;
  decision?: PilotReviewDecision;
  index: number;
  onDecision: (decision: PilotReviewDecision) => void;
}) {
  const { t } = useTranslation();
  const { segment } = comparison;
  return (
    <article className="overflow-hidden rounded-2xl border bg-card shadow-sm">
      <div className="border-b bg-muted/25 px-4 py-3 sm:px-5">
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-xs font-medium tabular-nums text-muted-foreground">
            #{index}
          </span>
          <Badge variant="secondary">
            {t(`pilot.category.${segment.category}`)}
          </Badge>
          {comparison.changed && (
            <Badge variant="outline">{t("pilot.outputChanged")}</Badge>
          )}
          {segment.hasPlaceholders && (
            <Badge variant="outline">{t("pilot.placeholders")}</Badge>
          )}
          <span className="ml-auto truncate text-xs text-muted-foreground">
            {segment.fileName} · {segment.jsonKey}
          </span>
        </div>
        <p className="mt-3 whitespace-pre-wrap text-[15px] leading-6">
          {segment.sourceText}
        </p>
        {segment.context && (
          <div className="mt-3 flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
            {segment.speaker && (
              <span>{t("pilot.speaker", { value: segment.speaker })}</span>
            )}
            {segment.sceneId && (
              <span>{t("pilot.scene", { value: segment.sceneId })}</span>
            )}
            {(segment.context.previous.length > 0 ||
              segment.context.following.length > 0) && (
              <span>
                {t("pilot.neighbors", {
                  before: segment.context.previous.length,
                  after: segment.context.following.length,
                })}
              </span>
            )}
          </div>
        )}
      </div>

      <div className="grid md:grid-cols-2">
        <VariantPanel
          label={t("pilot.baseline")}
          variant={comparison.baseline}
        />
        <VariantPanel
          accent
          label={t("pilot.contextual")}
          variant={comparison.contextual}
        />
      </div>

      <div className="flex flex-wrap items-center gap-2 border-t bg-muted/15 px-4 py-3 sm:px-5">
        <span className="mr-1 text-xs font-medium text-muted-foreground">
          {t("pilot.yourChoice")}
        </span>
        {DECISIONS.map((value) => (
          <Button
            aria-pressed={decision === value}
            className="h-7 text-xs"
            key={value}
            onClick={() => onDecision(value)}
            size="sm"
            variant={decision === value ? "secondary" : "ghost"}
          >
            {t(`pilot.decision.${value}`)}
          </Button>
        ))}
      </div>
    </article>
  );
}

function VariantPanel({
  accent = false,
  label,
  variant,
}: {
  accent?: boolean;
  label: string;
  variant: PilotVariantResult;
}) {
  const { t } = useTranslation();
  return (
    <div
      className={cn(
        "min-w-0 p-4 sm:p-5 md:first:border-r",
        accent && "bg-primary/[0.025]",
      )}
    >
      <div className="flex items-center justify-between gap-3">
        <p
          className={cn(
            "text-xs font-semibold uppercase tracking-wide",
            accent && "text-primary",
          )}
        >
          {label}
        </p>
        <Badge variant={variant.needsReview ? "destructive" : "secondary"}>
          {variant.needsReview ? t("pilot.review") : t("pilot.qaOk")} ·{" "}
          {variant.qa.score}
        </Badge>
      </div>
      <p className="mt-4 min-h-12 whitespace-pre-wrap text-sm leading-6">
        {variant.translatedText || t("pilot.empty")}
      </p>
      {(variant.qa.errors.length > 0 || variant.qualityFlags.length > 0) && (
        <div className="mt-4 flex flex-wrap gap-1.5">
          {variant.qa.errors.map((error, index) => (
            <Badge key={`${error.type}-${index}`} variant="destructive">
              {t(`pilot.qaError.${error.type}`)}
            </Badge>
          ))}
          {variant.qualityFlags.map((flag) => (
            <QualityFlagBadge flag={flag} key={flag} />
          ))}
        </div>
      )}
    </div>
  );
}

function QualityFlagBadge({ flag }: { flag: PilotQualityFlag }) {
  const { t } = useTranslation();
  return <Badge variant="destructive">{t(`pilot.qualityFlag.${flag}`)}</Badge>;
}

function formatDuration(durationMs: number): string {
  if (durationMs < 1_000) return `${durationMs} ms`;
  return `${(durationMs / 1_000).toFixed(1)} s`;
}

function formatTokenCount(value: number | null, language: string): string {
  if (value === null) return "— tokens";
  return `${new Intl.NumberFormat(language).format(value)} tokens`;
}
