import { describe, expect, it } from "vitest";
import { formatValueSummary } from "./typeHelpers";

describe("formatValueSummary", () => {
  it("shows object field values instead of only field names", () => {
    expect(
      formatValueSummary({ id: "entry-1", value: 0 }),
    ).toBe("{ id: entry-1, value: 0 }");
  });

  it("shows nested object values compactly", () => {
    expect(
      formatValueSummary({
        deep: { enabled: true, label: "alpha" },
        value: "beta",
      }),
    ).toBe("{ deep: { enabled: true, label: alpha }, value: beta }");
  });
});
