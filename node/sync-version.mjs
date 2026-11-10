#!/usr/bin/env node

// Syncs package.json / package-lock.json with the root Cargo workspace version.
// Pass --check to only verify that nothing has drifted.

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const checkOnly = process.argv.includes("--check");

const cargo = readFileSync(resolve(here, "../Cargo.toml"), "utf8");
const version = cargo.match(/\[workspace\.package\][\s\S]*?^version\s*=\s*"([^"]+)"/m)?.[1];
if (!version) throw new Error("Could not read [workspace.package] version from Cargo.toml");

function sync(file, update) {
  const path = resolve(here, file);
  const before = readFileSync(path, "utf8");
  const eol = before.includes("\r\n") ? "\r\n" : "\n";
  const json = JSON.parse(before);
  update(json);
  const after = `${JSON.stringify(json, null, 2)}\n`.replaceAll("\n", eol);
  if (before === after) return false;
  if (!checkOnly) writeFileSync(path, after);
  return true;
}

function setDeps(deps) {
  for (const name of Object.keys(deps ?? {})) deps[name] = version;
}

const changed = [
  sync("package.json", (pkg) => {
    pkg.version = version;
    setDeps(pkg.optionalDependencies);
  }) && "package.json",
  sync("package-lock.json", (lock) => {
    lock.version = version;
    const root = lock.packages[""];
    root.version = version;
    setDeps(root.optionalDependencies);
    for (const name of Object.keys(root.optionalDependencies ?? {})) {
      const entry = lock.packages[`node_modules/${name}`];
      if (!entry) continue;
      entry.version = version;
      entry.resolved = `https://registry.npmjs.org/${name}/-/${name}-${version}.tgz`;
      delete entry.integrity;
    }
  }) && "package-lock.json",
].filter(Boolean);

if (changed.length === 0) {
  console.log(`Version ${version} is in sync.`);
} else if (checkOnly) {
  console.error(`Out of sync with ${version}: ${changed.join(", ")}. Run: npm run sync-version`);
  process.exit(1);
} else {
  console.log(`Synced version ${version}: ${changed.join(", ")}`);
}
