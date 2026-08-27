import { create } from "zustand";
import type { PartOfSpeech, TerminologyEntryStatus } from "@/lib/types";

type Scope = "project" | "global";
interface TerminologyUiState {
  search: string;
  partOfSpeech: PartOfSpeech | "all";
  semanticType: string;
  status: TerminologyEntryStatus | "all";
  scope: Scope;
  page: number;
  selectedIds: string[];
  setSearch: (value: string) => void;
  setPartOfSpeech: (value: PartOfSpeech | "all") => void;
  setSemanticType: (value: string) => void;
  setStatus: (value: TerminologyEntryStatus | "all") => void;
  setScope: (value: Scope) => void;
  setPage: (value: number) => void;
  toggleSelected: (id: string) => void;
  clearSelection: () => void;
}
export const useTerminologyUiStore = create<TerminologyUiState>((set) => ({
  search: "",
  partOfSpeech: "all",
  semanticType: "",
  status: "active",
  scope: "project",
  page: 0,
  selectedIds: [],
  setSearch: (search) => set({ search, page: 0 }),
  setPartOfSpeech: (partOfSpeech) => set({ partOfSpeech, page: 0 }),
  setSemanticType: (semanticType) => set({ semanticType, page: 0 }),
  setStatus: (status) => set({ status, page: 0 }),
  setScope: (scope) => set({ scope, page: 0, selectedIds: [] }),
  setPage: (page) => set({ page }),
  toggleSelected: (id) =>
    set((state) => ({
      selectedIds: state.selectedIds.includes(id)
        ? state.selectedIds.filter((value) => value !== id)
        : [...state.selectedIds, id],
    })),
  clearSelection: () => set({ selectedIds: [] }),
}));
