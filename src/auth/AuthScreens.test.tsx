import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it, vi } from "vitest";

import { FirstRunSetup, LoginScreen } from "./AuthScreens";
import { UsernameHistoryScope } from "./usernameHistory";

describe("local authentication screens", () => {
  afterEach(() => vi.unstubAllGlobals());
  it("offers remembered usernames with the most recent selected and an empty password", () => {
    vi.stubGlobal("window", { localStorage: { getItem: () => '["recent.user","other.user"]' } });
    const html = renderToStaticMarkup(<UsernameHistoryScope.Provider value="test-history"><LoginScreen onAuthenticated={() => undefined} /></UsernameHistoryScope.Provider>);
    expect(html).toContain("Saved usernames");
    expect(html).toContain('value="recent.user" selected=""');
    expect(html).toContain("other.user");
    expect(html).toContain("Use another username");
    expect(html).toContain("Clear saved usernames");
    expect(html).toMatch(/type="password"[^>]*value=""/);
  });

  it("keeps manual login available without saved history", () => {
    const html = renderToStaticMarkup(<LoginScreen onAuthenticated={() => undefined} />);
    expect(html).toContain('autoComplete="username"');
    expect(html).not.toContain("Saved usernames");
  });
  it("renders first-run setup without factory credentials", () => {
    const html = renderToStaticMarkup(<FirstRunSetup onAuthenticated={() => undefined} />);
    expect(html).toContain("Create the first administrator account");
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
