import { useQuery } from "@tanstack/react-query";
import { textFitApi } from "@/lib/api/textFit";

/**
 * Load the game's main font (bytes from Rust) as a FontFace so previews are
 * drawn with it. Returns the CSS family name, or `null` when the font is not
 * available (previews then fall back to a monospace font; layout itself is
 * always computed in Rust).
 */
export async function loadGameFont(projectId: string): Promise<string | null> {
  if (
    typeof FontFace === "undefined" ||
    typeof document === "undefined" ||
    !document.fonts
  ) {
    return null;
  }
  const bytes = await textFitApi.gameFont(projectId);
  const family = `h2s-game-font-${projectId}`;
  const face = new FontFace(family, bytes);
  await face.load();
  document.fonts.add(face);
  return family;
}

export function useGameFont(projectId: string | null): string | null {
  const { data } = useQuery({
    queryKey: ["game-font", projectId],
    queryFn: () =>
      projectId ? loadGameFont(projectId) : Promise.resolve(null),
    enabled: projectId !== null,
    staleTime: Infinity,
    retry: false,
  });
  return data ?? null;
}
