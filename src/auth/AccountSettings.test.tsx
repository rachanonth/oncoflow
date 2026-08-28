import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { AccountSettings } from "./AccountSettings";
import { SessionIdentity } from "./SessionIdentity";

const user = { id: 7, username: "local.pharmacist", displayName: "เภสัชกรทดสอบ", role: "pharmacist" as const, userType: "pharmacist" as const };

describe("authenticated account UI", () => {
  it("shows the current pharmacist and logout control", () => {
    const html = renderToStaticMarkup(<SessionIdentity user={user} onLogout={() => undefined} />);
    expect(html).toContain("เภสัชกรทดสอบ");
    expect(html).toContain("Pharmacist");
    expect(html).toContain("Sign out");
  });

  it("shows account identity and local password change UI", () => {
    const html = renderToStaticMarkup(<AccountSettings user={user} />);
    expect(html).toContain("Current identity");
    expect(html).toContain("@local.pharmacist");
    expect(html).toContain("Change password");
    expect(html).toContain("not a digital signature");
  });
});
