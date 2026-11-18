import { invoke } from "@tauri-apps/api/core";
import type { UnpackReport } from "@/lib/types";

/** Decrypt a packed Wolf RPG game in its folder (plan UWF). */
export const wolfApi = {
  /** Extract the text archives, remove a Pro 3.5 protection, move the archives aside. */
  unpackGame: (gamePath: string) =>
    invoke<UnpackReport>("unpack_wolf_game", { gamePath }),
  /** Stop the running decryption at its next file; nothing is moved then. */
  cancelUnpack: () => invoke<void>("cancel_wolf_unpack"),
};
