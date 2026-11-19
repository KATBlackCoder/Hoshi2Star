import { create } from "zustand";
import { t } from "i18next";
import { toast } from "sonner";
import type { UnpackProgress, UnpackReport } from "@/lib/types";
import { wolfApi } from "@/lib/api/wolf";
import {
  WOLF_ARCHIVE_PRESENT,
  WOLF_PACKED,
  isWolfUnpackCancelled,
  openProjectErrorKey,
  wolfReadErrorDetail,
} from "@/lib/projectError";
import { errorMessage } from "@/lib/utils";
import { openProject } from "@/stores/project";

// ---------------------------------------------------------------------------
// Decrypt a packed Wolf RPG game before opening it (plan UWF)
// ---------------------------------------------------------------------------

/** Why the decryption is offered: no loose files yet, or text archives left. */
export type WolfUnpackReason = "packed" | "archivePresent";

/** `stopped`: the user stopped the decryption; it can be resumed. */
export type WolfUnpackPhase =
  | "confirm"
  | "running"
  | "done"
  | "stopped"
  | "error";

interface WolfUnpackState {
  /** The game folder; `null` while the dialog is closed. */
  gamePath: string | null;
  reason: WolfUnpackReason;
  /** The archives named by the backend. */
  archives: string | undefined;
  phase: WolfUnpackPhase;
  progress: UnpackProgress | null;
  report: UnpackReport | null;
  error: string | null;
  cancelling: boolean;
}

const CLOSED: WolfUnpackState = {
  gamePath: null,
  reason: "packed",
  archives: undefined,
  phase: "confirm",
  progress: null,
  report: null,
  error: null,
  cancelling: false,
};

export const useWolfUnpackStore = create<WolfUnpackState>()(() => CLOSED);

/** The archive paths an error names (`Data.wolf, Data/MapData.wolf`). */
function archivesIn(text: string): string | undefined {
  const found = [...new Set(text.match(/[\w./-]+\.wolf\b/gi) ?? [])];
  return found.length > 0 ? found.join(", ") : undefined;
}

/**
 * Offer the decryption when `err` says the Wolf game at `gamePath` is still
 * packed (`wolf_packed`) or keeps text archives (`wolf_archive_present`).
 * Returns `false` for any other error.
 */
export function offerWolfUnpack(gamePath: string, err: unknown): boolean {
  const text = String(err);
  const reason: WolfUnpackReason | null = text.includes(WOLF_PACKED)
    ? "packed"
    : text.includes(WOLF_ARCHIVE_PRESENT)
      ? "archivePresent"
      : null;
  if (!reason) return false;
  useWolfUnpackStore.setState({
    ...CLOSED,
    gamePath,
    reason,
    archives: archivesIn(text),
  });
  return true;
}

/**
 * Open the game folder chosen by the user. A Wolf game that must be decrypted
 * first opens the decryption dialog instead of an error toast.
 */
export async function openGameFolder(gamePath: string): Promise<void> {
  try {
    await openProject(gamePath);
  } catch (err) {
    if (offerWolfUnpack(gamePath, err)) return;
    toast.error(t(openProjectErrorKey(err)), {
      description: wolfReadErrorDetail(err),
    });
  }
}

/** Decrypt the game (or resume after a stop), then open it again. */
export async function startWolfUnpack(): Promise<void> {
  const { gamePath, phase } = useWolfUnpackStore.getState();
  if (!gamePath || phase === "running") return;
  useWolfUnpackStore.setState({
    phase: "running",
    progress: null,
    error: null,
    cancelling: false,
  });
  try {
    const report = await wolfApi.unpackGame(gamePath);
    useWolfUnpackStore.setState({ phase: "done", report, cancelling: false });
  } catch (err) {
    // A stop keeps the last progress, to say where it stopped.
    useWolfUnpackStore.setState(
      isWolfUnpackCancelled(err)
        ? { phase: "stopped", error: null, cancelling: false }
        : { phase: "error", error: errorMessage(err), cancelling: false },
    );
    return;
  }
  await openGameFolder(gamePath);
}

/** Ask the backend to stop at its next file. */
export async function cancelWolfUnpack(): Promise<void> {
  useWolfUnpackStore.setState({ cancelling: true });
  try {
    await wolfApi.cancelUnpack();
  } catch (err) {
    toast.error(errorMessage(err));
  }
}

/** Progress event of the running decryption. */
export function setWolfUnpackProgress(progress: UnpackProgress): void {
  if (useWolfUnpackStore.getState().phase !== "running") return;
  useWolfUnpackStore.setState({ progress });
}

/** Close the dialog; refused while the decryption runs. */
export function closeWolfUnpackDialog(): void {
  if (useWolfUnpackStore.getState().phase === "running") return;
  useWolfUnpackStore.setState(CLOSED);
}
