// LLM provider defaults — single TS source of truth.
// DEFAULT_OLLAMA_MODEL MUST match the Rust constant of the same name in
// src-tauri/src/llm/provider.rs (mirrored there for backend fallbacks).
export const DEFAULT_OLLAMA_URL = "http://localhost:11434/v1";
export const DEFAULT_OLLAMA_MODEL = "gemma4:e4b";
export const DEFAULT_BATCH_SIZE = 20;

export const RESOURCE_PROFILES = [
  { id: "eco", batchSize: 8 },
  { id: "balanced", batchSize: DEFAULT_BATCH_SIZE },
  { id: "fast", batchSize: 50 },
] as const;

export const PROVIDER_PRESETS = [
  {
    id: "ollama",
    label: "Ollama",
    url: DEFAULT_OLLAMA_URL,
    model: DEFAULT_OLLAMA_MODEL,
    requiresApiKey: false,
  },
  {
    id: "lmstudio",
    label: "LM Studio",
    url: "http://localhost:1234/v1",
    model: "",
    requiresApiKey: false,
  },
  {
    id: "huggingface",
    label: "Hugging Face",
    url: "https://router.huggingface.co/v1",
    model: "Qwen/Qwen3-8B",
    requiresApiKey: true,
  },
  {
    id: "openai",
    label: "OpenAI",
    url: "https://api.openai.com/v1",
    model: "",
    requiresApiKey: true,
  },
  {
    id: "custom",
    label: "Cloud / compatible",
    url: "",
    model: "",
    requiresApiKey: true,
  },
] as const;

export type ProviderId = (typeof PROVIDER_PRESETS)[number]["id"];

export const GAME_LANGUAGES = [
  { code: "ja", label: "日本語" },
  { code: "en", label: "English" },
  { code: "fr", label: "Français" },
  { code: "es", label: "Español" },
  { code: "de", label: "Deutsch" },
  { code: "it", label: "Italiano" },
  { code: "pt", label: "Português" },
  { code: "ko", label: "한국어" },
  { code: "zh", label: "中文" },
] as const;

// RPG Maker MV/MZ placeholder pattern — single source of truth.
// Used in columns.tsx (buildHighlightedNodes).
// Reset lastIndex before each exec() call since the flag /g is stateful.
export const PH_RE_SOURCE =
  /\\[+-]\w+\[\d+\]|\\[VNPCI]\[\d+\]|\\[G\\$.|!><^{}]|\[%\d+\]/g;

/** Returns a fresh RegExp clone so concurrent callers don't share lastIndex. */
export function clonePH_RE(): RegExp {
  return new RegExp(PH_RE_SOURCE.source, PH_RE_SOURCE.flags);
}

// Wolf RPG placeholder pattern — mirrors `RE_WOLF` in
// src-tauri/src/engines/wolf/placeholders.rs (same alternative order,
// minus the trailing literal-newline alternative which has no visual chip).
export const PH_RE_WOLF =
  /\\r\[[^[\],]+,[^[\]]*\]|\\(?:udb|cdb|sdb)\[\d+:\d+:\d+\]|\\sysS\[\d+\]|\\cself\[\d{1,2}\]|\\self\[\d\]|\\sys\[\d+\]|\\space\[\d+\]|\\v\?\[\d+\]|\\(?:sp|mx|my|ax|ay)\[\d+\]|\\-\[\d+\]|\\font\[\d\]|\\[vcsfiVCSFI]\[\d+\]|\\m\[\d+\]|<[LCR]>|\\A[+-]|\\[EN\\!.^><]/g;

/** Returns a fresh RegExp clone so concurrent callers don't share lastIndex. */
export function clonePH_RE_WOLF(): RegExp {
  return new RegExp(PH_RE_WOLF.source, PH_RE_WOLF.flags);
}

/** Returns a fresh placeholder regex matching the given project engine. */
export function getPlaceholderRegex(engine: string): RegExp {
  return engine === "wolf" ? clonePH_RE_WOLF() : clonePH_RE();
}
