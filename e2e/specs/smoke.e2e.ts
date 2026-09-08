import { browser, $, expect } from "@wdio/globals";

/**
 * The one smoke spec: the app launches, the Dashboard renders, and the typed IPC seam is live
 * (the backend-status line reaches a `pong`). Creating and booting an emulator is a follow-up
 * spec once this harness runs green in CI (see `.agent/tasks/0030`).
 */
describe("Emulator Studio — launch smoke", () => {
  it("shows the Dashboard with the sidebar and a live backend", async () => {
    // The app opens on `/` → the Dashboard, whose <Screen> renders an <h1> "My emulators".
    const heading = await $("h1");
    await expect(heading).toHaveText(expect.stringContaining("My emulators"));

    // The sidebar lists the nav — at least "Dashboard" and "Dependencies".
    await expect($("aside")).toHaveText(expect.stringContaining("Dependencies"));

    // The typed IPC seam is wired end to end: `usePing` resolves to a `pong, Emulator Studio`.
    const status = await $('[data-testid="backend-status"]');
    await browser.waitUntil(
      async () => (await status.getText()).toLowerCase().includes("pong"),
      { timeout: 30_000, timeoutMsg: "backend-status never reached a pong" },
    );
  });
});
