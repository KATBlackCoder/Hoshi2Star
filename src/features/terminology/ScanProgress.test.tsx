import { afterEach, describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import i18n from "@/lib/i18n";
import { ScanProgress } from "./ScanProgress";

const PROGRESS = {
  scanId: "scan-1",
  projectId: "p1",
  processed: 5,
  total: 10,
  discovered: 2,
};

afterEach(async () => {
  await i18n.changeLanguage("fr");
});

describe("ScanProgress (audit F6: translated aria-label)", () => {
  it("names the progress bar in the app language", async () => {
    render(<ScanProgress progress={PROGRESS} onCancel={() => {}} />);
    expect(
      screen.getByRole("progressbar", { name: "Progression de l’analyse" }),
    ).toBeInTheDocument();

    await i18n.changeLanguage("en");
    expect(
      await screen.findByRole("progressbar", { name: "Scan progress" }),
    ).toBeInTheDocument();
  });
});
