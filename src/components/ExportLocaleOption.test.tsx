import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { ExportLocaleOption } from "@/components/ExportLocaleOption";
import { ExportIntegrityDialog } from "@/components/ExportIntegrityDialog";
import type { LocaleOption } from "@/lib/types";

const OPTION: LocaleOption = {
  current: "ja_JP",
  target: "en_US",
  consequences: [
    "Sets System.json `locale` from ja_JP to en_US.",
    "Battle enemy letters switch from full-width (Ａ, Ｂ…) to half-width (A, B…).",
  ],
  uses: [
    {
      location: "js/rmmz_objects.js:7",
      code: "return $gameSystem.isCJK() ? A : B;",
    },
  ],
};

describe("ExportLocaleOption", () => {
  it("shows every consequence and the code lines behind them", async () => {
    const onCheckedChange = vi.fn();
    render(
      <ExportLocaleOption
        option={OPTION}
        checked={false}
        onCheckedChange={onCheckedChange}
      />,
    );

    const checkbox = screen.getByRole("checkbox", { name: /ja_JP → en_US/ });
    expect(checkbox).not.toBeChecked();
    for (const line of OPTION.consequences) {
      expect(screen.getByText(line)).toBeInTheDocument();
    }
    expect(screen.getByText("js/rmmz_objects.js:7")).toBeInTheDocument();

    await userEvent.click(checkbox);
    expect(onCheckedChange).toHaveBeenCalledWith(true);
  });
});

describe("ExportIntegrityDialog", () => {
  it("lists the problems and closes", async () => {
    const onClose = vi.fn();
    render(
      <ExportIntegrityDialog
        issues={["js/plugins.js line 4: unplanned change"]}
        onClose={onClose}
      />,
    );

    expect(
      screen.getByText("js/plugins.js line 4: unplanned change"),
    ).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button"));
    expect(onClose).toHaveBeenCalled();
  });

  it("is hidden without problems", () => {
    render(<ExportIntegrityDialog issues={null} onClose={vi.fn()} />);
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
  });
});
