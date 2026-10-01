import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { ServerControls, ServerStatusDetails } from "./ServerControls";
import type { ServerStatus } from "../api/transport";

const online: ServerStatus = { mode: "host", running: true, databaseReady: true, signedInConnections: 3, addresses: ["192.168.1.10"], port: 7443, dataDirectory: "C:\\SyntheticHost", uptimeSeconds: 125, message: "Server online · database ready" };
describe("server status and host controls", () => {
  it("shows the LAN endpoint, database status, uptime, and signed-in connections", () => {
    const html = renderToStaticMarkup(<ServerStatusDetails status={online} />);
    expect(html).toContain("192.168.1.10:7443");
    expect(html).toContain("Server online");
    expect(html).toContain("3 (includes this PC");
    expect(html).toContain("2 minutes");
  });
  it("requires explicit shutdown confirmation when closing the host app", () => {
    const html = renderToStaticMarkup(<ServerControls status={online} requestExit onClose={() => undefined} onStatus={() => undefined} />);
    expect(html).toContain("Stop server and exit");
    expect(html).toContain("Keep running");
    expect(html).toContain("disconnects all users");
  });
  it("offers start when stopped and never exposes host controls on a client", () => {
    const stopped = renderToStaticMarkup(<ServerControls status={{ ...online, running: false, databaseReady: false }} requestExit={false} onClose={() => undefined} onStatus={() => undefined} />);
    expect(stopped).toContain("Start server");
    const remote = renderToStaticMarkup(<ServerControls status={{ ...online, mode: "client" }} requestExit={false} onClose={() => undefined} onStatus={() => undefined} />);
    expect(remote).not.toContain("Stop server");
    expect(remote).not.toContain("Export certificate");
  });
});
