#!/usr/bin/env node
// Validate schemas/emuprofile/v1.schema.json and its fixtures.
// Uses ajv if available (added as a dev dep in M0 task 0007); otherwise does a
// minimal structural check so this script is still useful on a bare checkout.

import { readFileSync, readdirSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const SCHEMA_DIR = join(ROOT, "schemas/emuprofile");
const schema = JSON.parse(readFileSync(join(SCHEMA_DIR, "v1.schema.json"), "utf8"));
const readJson = (p) => JSON.parse(readFileSync(p, "utf8"));
const list = (d) => (existsSync(d) ? readdirSync(d).filter((f) => f.endsWith(".json")) : []);

let validate;
let ajvMode = true;
try {
  const { default: Ajv } = await import("ajv");
  const ajv = new Ajv({ allErrors: true, strict: false });
  validate = ajv.compile(schema);
} catch {
  ajvMode = false;
  // Minimal fallback: can only confirm required keys + the platform const.
  // It cannot judge enum / additionalProperties, so invalid fixtures are skipped.
  validate = (data) => {
    const missing = (schema.required ?? []).filter((k) => !(k in data));
    validate.errors = missing.map((k) => ({ message: `missing required '${k}'` }));
    if (data.platform && data.platform !== "android")
      validate.errors.push({ message: `platform must be 'android'` });
    return validate.errors.length === 0;
  };
}

let failures = 0;
const check = (file, expectValid) => {
  const ok = validate(readJson(file));
  if (ok !== expectValid) {
    failures++;
    console.error(`  FAIL ${file} — expected ${expectValid ? "VALID" : "INVALID"}`);
    if (!ok) for (const e of validate.errors ?? []) console.error(`       ${e.instancePath ?? ""} ${e.message}`);
  } else {
    console.log(`  ok   ${file}`);
  }
};

console.log(`validate-schema: ${ajvMode ? "ajv" : "MINIMAL (install ajv for full checks)"} mode`);
console.log("valid fixtures (must pass):");
for (const f of list(join(SCHEMA_DIR, "fixtures/valid"))) check(join(SCHEMA_DIR, "fixtures/valid", f), true);
if (ajvMode) {
  console.log("invalid fixtures (must fail):");
  for (const f of list(join(SCHEMA_DIR, "fixtures/invalid"))) check(join(SCHEMA_DIR, "fixtures/invalid", f), false);
} else {
  console.log("invalid fixtures: SKIPPED (need ajv)");
}

if (failures) {
  console.error(`validate-schema: ${failures} failure(s).`);
  process.exit(1);
}
console.log(`validate-schema: ok${ajvMode ? "" : " (minimal — CI runs the full check)"}`);
