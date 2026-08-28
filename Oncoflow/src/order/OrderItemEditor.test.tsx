import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type { OrderLookups } from "../types/order";
import { diluentVolumeFromMaster, OrderItemEditor } from "./OrderItemEditor";

const lookups: OrderLookups = {
  patients: [],
  regimens: [],
  drugs: [{ id: 1, label: "Synthetic drug" }],
  routes: [{ id: 1, label: "IV" }],
  diluents: [{ id: 1, label: "NSS", volumeMl: 100 }],
  doctors: [],
  wards: [],
  preparationPharmacists: [],
};

describe("OrderItemEditor", () => {
  it("renders the simplified Drug form and all 24 hourly schedule choices", () => {
    const html = renderToStaticMarkup(<OrderItemEditor lookups={lookups} onCancel={() => undefined} onSave={async () => undefined} />);

    expect(html).toContain("Add drug");
    expect(html).toContain("Dose");
    expect(html).toContain("<span>mg</span>");
    expect(html).toContain("NSS (100 mL)");
    expect(html).toContain("Minutes");
    expect(html).toContain("Hours");
    expect(html.match(/value="\d{2}:00"/g)).toHaveLength(24);
    expect(html).not.toContain("Raw dose");
    expect(html).not.toContain("Legacy quantity");
    expect(html).not.toContain("Legacy missing flag");
    expect(html).not.toContain("drug line");
    expect(html).toContain('aria-label="Save drug"');
    expect(html).not.toContain(">Save drug</button>");
  });

  it("uses the selected diluent master volume as the auto-fill value", () => {
    expect(diluentVolumeFromMaster("1", lookups.diluents)).toBe("100");
    expect(diluentVolumeFromMaster("", lookups.diluents)).toBe("");
  });
});
