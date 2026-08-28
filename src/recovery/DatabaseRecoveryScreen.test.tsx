import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { DatabaseRecoveryScreen } from "./DatabaseRecoveryScreen";

describe("DatabaseRecoveryScreen", () => {
  it("presents recovery actions without silently replacing a damaged database", () => {
    const html = renderToStaticMarkup(<DatabaseRecoveryScreen status={{ databaseReady: false, databaseLocation: "C:\\Synthetic\\oncoflow.db", issue: { code: "database_corrupt", title: "Database integrity problem", message: "Synthetic safe recovery message." } }} onReady={() => undefined}/>);
    expect(html).toContain("Database integrity problem");
    expect(html).toContain("Retry database");
    expect(html).toContain("Select backup");
    expect(html).toContain("Open data folder");
    expect(html).toContain("was not replaced with an empty file");
  });
});
