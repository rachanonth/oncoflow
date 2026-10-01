import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { DrugForm } from "./DrugForm";

describe("DrugForm", () => {
  it("uses minutes/hours for default rate and only hours/days for expiry", () => {
    const html = renderToStaticMarkup(
      <DrugForm onCancel={() => undefined} onSaved={() => undefined} />,
    );

    expect(html).toContain('aria-label="Default rate unit"');
    expect(html).toContain('aria-label="Expiry time unit"');
    expect(html.match(/>Minutes<\/button>/g)).toHaveLength(1);
    expect(html.match(/>Hours<\/button>/g)).toHaveLength(2);
    expect(html.match(/>Days<\/button>/g)).toHaveLength(1);
  });
});
