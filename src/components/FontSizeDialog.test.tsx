import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { FontSizeDialog } from "@/components/FontSizeDialog";
import i18n from "@/lib/i18n";
import type { FontScanResult } from "@/lib/types";

const WOLF: FontScanResult = {
  existingFontCount: 0,
  totalTranslated: 32,
  engine: "wolf",
  gameFontSize: 12,
};

function renderDialog(scan: FontScanResult) {
  const onApply = vi.fn();
  render(
    <FontSizeDialog open scan={scan} onApply={onApply} onSkip={vi.fn()} />,
  );
  return onApply;
}

describe("FontSizeDialog", () => {
  it("proposes the Wolf game's own font size", async () => {
    const onApply = renderDialog(WOLF);
    expect(screen.getByRole("spinbutton")).toHaveValue(12);
    expect(
      screen.getByText(i18n.t("fontSizeDialog.hintWolfGame", { size: 12 })),
    ).toBeInTheDocument();
    await userEvent.click(
      screen.getByRole("button", { name: i18n.t("fontSizeDialog.apply") }),
    );
    expect(onApply).toHaveBeenCalledWith(12, true);
  });

  it("proposes no size when the Wolf game's size is unknown", async () => {
    const onApply = renderDialog({ ...WOLF, gameFontSize: null });
    expect(screen.getByRole("spinbutton")).toHaveValue(null);
    expect(
      screen.getByText(i18n.t("fontSizeDialog.hintWolfUnknown")),
    ).toBeInTheDocument();
    const apply = screen.getByRole("button", {
      name: i18n.t("fontSizeDialog.apply"),
    });
    expect(apply).toBeDisabled();
    await userEvent.type(screen.getByRole("spinbutton"), "14");
    expect(apply).toBeEnabled();
    await userEvent.click(apply);
    expect(onApply).toHaveBeenCalledWith(14, true);
  });

  it("keeps 18 for RPG Maker MV/MZ", () => {
    renderDialog({ ...WOLF, engine: "mv_mz", gameFontSize: null });
    expect(screen.getByRole("spinbutton")).toHaveValue(18);
  });
});
