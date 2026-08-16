import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { Button } from "../button";

describe("Button", () => {
  it.each(["accent", "brand"] as const)(
    "keeps the %s variant solid without a decorative glow",
    (variant) => {
      render(<Button variant={variant}>{variant}</Button>);

      const button = screen.getByRole("button", { name: variant });
      expect(button).not.toHaveClass("shadow-glow");
      expect(button).toHaveClass("focus-visible:ring-2");
    },
  );
});
