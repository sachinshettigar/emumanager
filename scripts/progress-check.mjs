#!/usr/bin/env node
// Consistency check for the progress-tracking files. Zero dependencies.
// Fails (exit 1) if .agent/state.json disagrees with the task files or MILESTONES.md.
// Run via `just progress`.

import { readFileSync, readdirSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const errors = [];
const err = (m) => errors.push(m);

const MILESTONE_STATUS = new Set(["todo", "in_progress", "done"]);
const TASK_STATUS = new Set(["todo", "doing", "review", "done", "blocked"]);

// --- load state.json --------------------------------------------------------
let state;
try {
  state = JSON.parse(readFileSync(join(ROOT, ".agent/state.json"), "utf8"));
} catch (e) {
  console.error(`FATAL: cannot read/parse .agent/state.json: ${e.message}`);
  process.exit(1);
}

// --- shape checks ---------------------------------------------------------------
if (state.project !== "emumanager") err(`state.project must be "emumanager"`);
if (!/^\d{4}-\d{2}-\d{2}$/.test(state.updated ?? "")) err(`state.updated must be YYYY-MM-DD`);
if (!/^M\d+$/.test(state.currentMilestone ?? "")) err(`state.currentMilestone must match ^M\\d+$`);
if (typeof state.milestones !== "object") err(`state.milestones missing`);
if (!Array.isArray(state.tasks)) err(`state.tasks must be an array`);

// --- milestones --------------------------------------------------------------
const milestoneIds = Object.keys(state.milestones ?? {});
for (const [id, m] of Object.entries(state.milestones ?? {})) {
  if (!/^M\d+$/.test(id)) err(`milestone key "${id}" must match ^M\\d+$`);
  if (!m || typeof m.title !== "string" || !m.title) err(`milestone ${id}: missing title`);
  if (!MILESTONE_STATUS.has(m?.status)) err(`milestone ${id}: bad status "${m?.status}"`);
}
if (state.currentMilestone && !milestoneIds.includes(state.currentMilestone))
  err(`currentMilestone ${state.currentMilestone} not present in state.milestones`);

const inProgress = Object.entries(state.milestones ?? {}).filter(([, m]) => m.status === "in_progress");
if (inProgress.length > 1)
  err(`more than one milestone is in_progress: ${inProgress.map(([k]) => k).join(", ")}`);
if (state.currentMilestone && state.milestones?.[state.currentMilestone]?.status === "done")
  err(`currentMilestone ${state.currentMilestone} is marked done — advance currentMilestone`);

// --- tasks <-> task files -----------------------------------------------------
const tasksDir = join(ROOT, ".agent/tasks");
const taskFiles = existsSync(tasksDir)
  ? readdirSync(tasksDir).filter((f) => /^\d{4}-.*\.md$/.test(f))
  : [];
const fileById = new Map();
for (const f of taskFiles) {
  const idMatch = f.match(/^(\d{4})-/);
  if (idMatch) fileById.set(idMatch[1], f);
}

const seen = new Set();
let doingCount = 0;
for (const t of state.tasks ?? []) {
  if (!/^\d{4}$/.test(t.id ?? "")) { err(`task id "${t.id}" must be 4 digits`); continue; }
  if (seen.has(t.id)) err(`duplicate task id ${t.id} in state.json`);
  seen.add(t.id);
  if (!TASK_STATUS.has(t.status)) err(`task ${t.id}: bad status "${t.status}"`);
  if (t.status === "doing") doingCount++;
  if (t.milestone && !milestoneIds.includes(t.milestone))
    err(`task ${t.id}: milestone ${t.milestone} not in state.milestones`);

  const file = fileById.get(t.id);
  if (!file) { err(`task ${t.id} ("${t.title}") has no file .agent/tasks/${t.id}-*.md`); continue; }

  const body = readFileSync(join(tasksDir, file), "utf8");
  const fm = body.match(/^---\r?\n([\s\S]*?)\r?\n---/);
  if (!fm) { err(`task file ${file}: missing front matter`); continue; }
  const fmStatus = fm[1].match(/^status:\s*"?([a-z_]+)"?\s*$/m)?.[1];
  const fmId = fm[1].match(/^id:\s*"?(\d{4})"?\s*$/m)?.[1];
  if (fmId !== t.id) err(`task file ${file}: front-matter id "${fmId}" != state.json id "${t.id}"`);
  if (fmStatus !== t.status)
    err(`task ${t.id}: status mismatch — state.json="${t.status}" task file="${fmStatus}"`);
}
if (doingCount > 1) err(`${doingCount} tasks are "doing" — keep it to one at a time`);

for (const [id, file] of fileById) {
  if (!seen.has(id)) err(`task file ${file} has no entry in state.json tasks[]`);
}

// --- done milestones must have all MILESTONES.md boxes ticked ----------------
const mdPath = join(ROOT, "MILESTONES.md");
if (existsSync(mdPath)) {
  const md = readFileSync(mdPath, "utf8");
  for (const [id, m] of Object.entries(state.milestones ?? {})) {
    if (m.status !== "done") continue;
    const re = new RegExp(`^##\\s+${id}\\b[\\s\\S]*?(?=^##\\s|\\Z)`, "m");
    const section = md.match(re)?.[0];
    if (!section) { err(`MILESTONES.md has no "## ${id}" section but state says it is done`); continue; }
    if (/^- \[ \]/m.test(section))
      err(`milestone ${id} is "done" in state.json but MILESTONES.md still has unchecked boxes`);
  }
} else {
  err(`MILESTONES.md not found`);
}

// --- report ------------------------------------------------------------------
if (errors.length) {
  console.error(`progress-check: ${errors.length} problem(s):`);
  for (const e of errors) console.error(`  - ${e}`);
  process.exit(1);
}
console.log(
  `progress-check: ok — milestone ${state.currentMilestone}, ` +
    `${state.tasks.length} task(s), ${taskFiles.length} task file(s).`,
);
