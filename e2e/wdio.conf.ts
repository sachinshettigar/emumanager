import { spawn, type ChildProcess } from "node:child_process";
import { existsSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, "..");
const isWindows = process.platform === "win32";

/**
 * The debug binary `scripts/e2e.sh` builds with `pnpm tauri build --debug`.
 *
 * Tauri names the executable from `tauri.conf.json`'s `mainBinaryName` (unset here) → the Cargo
 * package name (`emumanager`), *not* the `productName`. A couple of fallbacks are tried so a
 * future `mainBinaryName` or crate rename doesn't silently break this.
 */
function appBinaryPath(): string {
  const ext = isWindows ? ".exe" : "";
  const candidates = ["emumanager", "emulator-studio", "Emulator Studio"].map((name) =>
    join(repoRoot, "src-tauri", "target", "debug", `${name}${ext}`),
  );
  const found = candidates.find((p) => existsSync(p));
  if (!found) {
    throw new Error(
      `no debug app binary found — run \`pnpm tauri build --debug\` first. Looked for:\n  ${candidates.join(
        "\n  ",
      )}`,
    );
  }
  return found;
}

/** `tauri-driver` is a cargo-installed binary (`cargo install tauri-driver`). */
function tauriDriverBin(): string {
  const ext = isWindows ? ".exe" : "";
  const cargoBin = join(homedir(), ".cargo", "bin", `tauri-driver${ext}`);
  return existsSync(cargoBin) ? cargoBin : `tauri-driver${ext}`; // else rely on PATH
}

let tauriDriver: ChildProcess | undefined;

export const config: WebdriverIO.Config = {
  runner: "local",
  tsConfigPath: join(here, "tsconfig.json"),

  specs: ["./specs/**/*.e2e.ts"],
  maxInstances: 1,

  capabilities: [
    {
      // eslint-disable-next-line @typescript-eslint/naming-convention
      "tauri:options": { application: appBinaryPath() },
    } as WebdriverIO.Capabilities,
  ],

  hostname: "127.0.0.1",
  port: 4444,
  path: "/",

  logLevel: "info",
  waitforTimeout: 20_000,
  connectionRetryTimeout: 120_000,
  connectionRetryCount: 3,

  framework: "mocha",
  mochaOpts: { ui: "bdd", timeout: 120_000 },
  reporters: ["spec"],

  // `tauri-driver` proxies WebDriver to the platform webview driver — spawn it per session.
  beforeSession() {
    tauriDriver = spawn(tauriDriverBin(), [], {
      stdio: [null, process.stdout, process.stderr],
    });
  },
  afterSession() {
    tauriDriver?.kill();
    tauriDriver = undefined;
  },
};
