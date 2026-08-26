import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Button } from "@/components/ui/button";

describe("Button", () => {
  it("keeps an accessible target and press feedback", () => {
    render(<Button>Ouvrir</Button>);
    expect(screen.getByRole("button")).toHaveClass(
      "min-h-10",
      "active:not-aria-[haspopup]:scale-[0.96]",
    );
  });

  it("can disable press scaling for destructive or static layouts", () => {
    render(<Button static>Supprimer</Button>);
    expect(screen.getByRole("button")).not.toHaveClass(
      "active:not-aria-[haspopup]:scale-[0.96]",
    );
  });
});
