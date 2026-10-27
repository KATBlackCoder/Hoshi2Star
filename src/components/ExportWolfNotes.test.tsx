import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";

import { ExportWolfNotes } from "@/components/ExportWolfNotes";
import type { WolfExportPreflight } from "@/lib/types";

const NOTES: WolfExportPreflight = {
  archives: ["Data.wolf"],
  shiftJis: true,
  accentedTexts: 3,
  pathWarning: "path holds 〜",
};

describe("ExportWolfNotes", () => {
  it("announces the rebuilt archive, the folded accents and the path warning", () => {
    render(<ExportWolfNotes notes={NOTES} />);
    expect(screen.getByText(/Data\.wolf/)).toBeInTheDocument();
    expect(screen.getByText(/3/)).toBeInTheDocument();
    expect(screen.getByRole("alert")).toBeInTheDocument();
  });

  it("warns about the font instead of folding for a UTF-8 game", () => {
    const { container } = render(
      <ExportWolfNotes
        notes={{ ...NOTES, shiftJis: false, pathWarning: null }}
      />,
    );
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(container.querySelectorAll("li")).toHaveLength(2);
  });

  it("renders nothing when there is nothing to say", () => {
    const { container } = render(
      <ExportWolfNotes
        notes={{
          archives: [],
          shiftJis: false,
          accentedTexts: 0,
          pathWarning: null,
        }}
      />,
    );
    expect(container).toBeEmptyDOMElement();
  });
});
