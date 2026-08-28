import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { BuddhistDateInput, BuddhistDateTimeInput } from "./BuddhistDateInput";

describe("Buddhist calendar inputs", () => {
  it("shows a read-only Buddhist date while retaining a controlled ISO value", () => {
    const html = renderToStaticMarkup(<BuddhistDateInput value="2026-08-25" onChange={() => undefined} />);
    expect(html).toContain('value="25/08/2569"');
    expect(html).toContain("readOnly");
    expect(html).toContain('aria-haspopup="dialog"');
    expect(html).not.toContain('type="date"');
  });

  it("shows Bangkok date-time in 24-hour format without a native date-time input", () => {
    const html = renderToStaticMarkup(<BuddhistDateTimeInput value="2026-08-25T09:07" onChange={() => undefined} />);
    expect(html).toContain('value="25/08/2569 09:07"');
    expect(html).not.toContain("datetime-local");
  });
});
