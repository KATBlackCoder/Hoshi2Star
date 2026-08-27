import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";
import { ScanProgress } from "./ScanProgress";

it("announces bounded progress and exposes cancellation", async () => {
  const cancel = vi.fn();
  render(
    <ScanProgress
      progress={{
        scanId: "scan",
        projectId: "p1",
        processed: 25,
        total: 100,
        discovered: 8,
      }}
      onCancel={cancel}
    />,
  );
  expect(screen.getByRole("progressbar")).toHaveAttribute(
    "aria-valuenow",
    "25",
  );
  expect(screen.getByText("25/100 · 8 termes")).toHaveClass("tabular-nums");
  await userEvent.click(
    screen.getByRole("button", { name: "Annuler l’analyse du vocabulaire" }),
  );
  expect(cancel).toHaveBeenCalledOnce();
});
