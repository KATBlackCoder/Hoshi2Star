import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { mockIPC } from "@tauri-apps/api/mocks";

const toastError = vi.fn();
vi.mock("sonner", () => ({
  toast: {
    error: (...args: unknown[]) => toastError(...args),
    success: vi.fn(),
    info: vi.fn(),
    warning: vi.fn(),
  },
}));

import { WolfUnpackDialog } from "@/components/WolfUnpackDialog";
import {
  openGameFolder,
  setWolfUnpackProgress,
  useWolfUnpackStore,
} from "@/stores/wolfUnpack";
import { useProjectStore } from "@/stores/project";
import type { Project, UnpackReport } from "@/lib/types";

const GAME = "/games/wolf";
const PACKED_ERROR =
  "wolf_packed: Data/MapData.wolf, Data/BasicData.wolf: this Wolf RPG game is still packed in .wolf archives; decrypt it before opening it";

const PROJECT: Project = {
  id: "w1",
  name: "Wolf",
  engine: "wolf",
  gamePath: GAME,
  sourceLang: "ja",
  targetLang: "fr",
  createdAt: "2026-11-18 00:00:00",
  updatedAt: "2026-11-18 00:00:00",
} as Project;

const REPORT: UnpackReport = {
  extracted: ["Data/BasicData.wolf", "Data/MapData.wolf"],
  alreadyUnpacked: [],
  skipped: [],
  failed: [],
  protection: "removedV35",
  unprotectedFiles: ["Game.dat", "CommonEvent.dat", "DataBase.dat"],
  moved: ["Data/MapData.wolf", "Data/BasicData.wolf"],
  warnings: [],
};

const initialProjectState = useProjectStore.getState();

beforeEach(() => {
  toastError.mockClear();
  useProjectStore.setState(initialProjectState, true);
  useWolfUnpackStore.setState({
    gamePath: null,
    reason: "packed",
    archives: undefined,
    phase: "confirm",
    progress: null,
    report: null,
    error: null,
    cancelling: false,
  });
});

describe("WolfUnpackDialog", () => {
  it("offers the decryption of a packed game, runs it and opens the game again", async () => {
    const calls: { cmd: string; args: unknown }[] = [];
    let unpacked = false;
    mockIPC((cmd, args) => {
      calls.push({ cmd, args });
      switch (cmd) {
        case "open_project":
          if (!unpacked) throw new Error(PACKED_ERROR);
          return { project: PROJECT, wasRestored: false };
        case "unpack_wolf_game":
          unpacked = true;
          return REPORT;
        case "get_source_files":
          return [];
        case "get_project_stats":
          return null;
        default:
          return null;
      }
    });
    render(<WolfUnpackDialog />);

    await openGameFolder(GAME);
    expect(toastError).not.toHaveBeenCalled();
    expect(
      await screen.findByText("Déchiffrer ce jeu Wolf RPG ?"),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Data\/MapData\.wolf, Data\/BasicData\.wolf/),
    ).toBeInTheDocument();
    expect(screen.getByText(/jamais supprimées/)).toBeInTheDocument();
    expect(screen.getByText(/travailler sur une copie/)).toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: "Déchiffrer" }));

    expect(await screen.findByText("Jeu déchiffré")).toBeInTheDocument();
    expect(
      screen.getByText(/Protection Wolf RPG Pro 3\.5 levée \(3 fichier/),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        "Mises de côté dans Hoshi2Star-archives/ : Data/MapData.wolf, Data/BasicData.wolf",
      ),
    ).toBeInTheDocument();
    expect(calls.find((c) => c.cmd === "unpack_wolf_game")?.args).toEqual({
      gamePath: GAME,
    });
    await waitFor(() =>
      expect(useProjectStore.getState().activeProjectId).toBe("w1"),
    );
    expect(calls.filter((c) => c.cmd === "open_project")).toHaveLength(2);
  });

  it("offers to finish the decryption when text archives are left", async () => {
    mockIPC((cmd) => {
      if (cmd === "open_project") {
        throw new Error(
          "wolf_archive_present: Data.wolf still in the game folder: the game reads these archives",
        );
      }
      return null;
    });
    render(<WolfUnpackDialog />);

    await openGameFolder(GAME);
    expect(
      await screen.findByText(/garde des archives de texte \(Data\.wolf\)/),
    ).toBeInTheDocument();
  });

  it("keeps the error toast for any other failure", async () => {
    mockIPC((cmd) => {
      if (cmd === "open_project")
        throw new Error("could not identify game engine");
      return null;
    });
    render(<WolfUnpackDialog />);

    await openGameFolder(GAME);
    expect(toastError).toHaveBeenCalledTimes(1);
    expect(useWolfUnpackStore.getState().gamePath).toBeNull();
  });

  it("shows the progress, stops on request and explains the refusal", async () => {
    let reject: (error: Error) => void = () => {};
    const commands: string[] = [];
    mockIPC((cmd) => {
      commands.push(cmd);
      if (cmd === "unpack_wolf_game") {
        return new Promise((_, rej) => {
          reject = rej;
        });
      }
      return null;
    });
    useWolfUnpackStore.setState({ gamePath: GAME, reason: "packed" });
    render(<WolfUnpackDialog />);

    await userEvent.click(screen.getByRole("button", { name: "Déchiffrer" }));
    expect(
      await screen.findByText("Déchiffrement en cours…"),
    ).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Fermer" })).toBeNull();

    setWolfUnpackProgress({
      archive: "Data/MapData.wolf",
      index: 3,
      total: 12,
    });
    expect(
      await screen.findByText("Data/MapData.wolf : fichier 3 sur 12"),
    ).toBeInTheDocument();
    expect(screen.getByRole("progressbar")).toHaveAttribute(
      "aria-valuenow",
      "3",
    );

    await userEvent.click(screen.getByRole("button", { name: "Arrêter" }));
    await waitFor(() => expect(commands).toContain("cancel_wolf_unpack"));
    expect(
      screen.getByRole("button", { name: "Arrêt en cours…" }),
    ).toBeDisabled();

    reject(
      new Error(
        "wolf_unpack_cancelled: the decryption was cancelled; no archive was moved",
      ),
    );
    expect(
      await screen.findByText("Déchiffrement impossible"),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        /Déchiffrement arrêté\. Aucune archive n'a été déplacée/,
      ),
    ).toBeInTheDocument();
    expect(commands.filter((c) => c === "open_project")).toHaveLength(0);

    await userEvent.click(screen.getByRole("button", { name: "Fermer" }));
    expect(useWolfUnpackStore.getState().gamePath).toBeNull();
  });

  it("names a Pro protection it cannot remove", async () => {
    mockIPC((cmd) => {
      if (cmd === "unpack_wolf_game") {
        throw new Error(
          "wolf_unpack_pro_unsupported: this Wolf RPG Pro protection (V2) cannot be removed; decrypt the game with UberWolf",
        );
      }
      return null;
    });
    useWolfUnpackStore.setState({ gamePath: GAME, reason: "packed" });
    render(<WolfUnpackDialog />);

    await userEvent.click(screen.getByRole("button", { name: "Déchiffrer" }));
    expect(
      await screen.findByText(/ne sait pas lever cette protection/),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/this Wolf RPG Pro protection \(V2\) cannot be removed/),
    ).toBeInTheDocument();
  });
});
