import type { PartOfSpeech } from "@/lib/types";

export const PART_OF_SPEECH_VALUES: PartOfSpeech[] = [
  "noun",
  "proper_noun",
  "verb",
  "adjective",
  "adverb",
  "expression",
  "unknown",
];

export const SEMANTIC_TYPE_VALUES = [
  "general",
  "character",
  "speaker",
  "class",
  "item",
  "weapon",
  "armor",
  "skill",
  "enemy",
  "state",
  "place",
  "event",
  "system",
  "title",
] as const;

export function includeCurrentOption(
  values: readonly string[],
  current: string,
): string[] {
  return values.includes(current) ? [...values] : [...values, current];
}
