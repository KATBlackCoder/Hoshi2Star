export const terminologyKeys = {
  all: ["terminology"] as const,
  list: (query: object) => [...terminologyKeys.all, "list", query] as const,
  stats: (
    sourceLanguage: string,
    targetLanguage: string,
    projectId: string | null,
  ) =>
    [
      ...terminologyKeys.all,
      "stats",
      sourceLanguage,
      targetLanguage,
      projectId,
    ] as const,
};
