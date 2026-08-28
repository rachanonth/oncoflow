import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { FirstRunSetup, LoginScreen } from "./AuthScreens";

describe("local authentication screens", () => {
  it("renders first-run setup without factory credentials", () => {
    const html = renderToStaticMarkup(<FirstRunSetup onAuthenticated={() => undefined} />);
    expect(html).toContain("Create the first local account");
    expect(html).toContain("Legacy Access passwords are never accepted");
    expect(html).toContain("No factory password is created");
    expect(html).not.toContain("admin/admin");
  });

  it("renders a generic failed-login state without the removed login copy", () => {
    const html = renderToStaticMarkup(<LoginScreen initialError="The local login was not accepted." onAuthenticated={() => undefined} />);
    expect(html).toContain("The local login was not accepted");
    expect(html).toContain("Chemotherapy preparation");
    expect(html).not.toContain("Local chemotherapy preparation");
    expect(html).not.toContain("Local sign in");
    expect(html).not.toContain("Welcome back");
    expect(html).not.toContain("Sign in to attribute ordering");
    expect(html).not.toContain("Authentication is offline");
    expect(html).not.toContain("password_hash");
  });
});
