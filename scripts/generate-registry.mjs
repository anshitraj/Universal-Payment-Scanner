#!/usr/bin/env node
// Generates registry/schemes/<id>.json and registry/schemes.json from the live Rust core's own
// scheme metadata (`upqr-core schemes`) - the same JSON already exposed to every binding via
// `Scanner::schemes()`. This script only reshapes and enriches that data; it never invents a fact
// the core doesn't already assert. See registry/README.md for what each derived field means and
// exactly which existing field it comes from.
//
// Usage: node scripts/generate-registry.mjs [--check]
//   --check   Don't write anything. Exit 1 if the committed registry/ files would change, so this
//             can run in CI to catch a registry that's drifted from the core it's supposed to mirror.

import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync, readdirSync, rmSync, existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const checkOnly = process.argv.includes("--check");

function sh(cmd, args) {
  return execFileSync(cmd, args, { cwd: repoRoot, encoding: "utf8" }).trim();
}

// --- 1. Rebuild and run the CLI's existing `schemes` command - the single source of truth. -----

console.log("Building upqr-core (release)...");
execFileSync("cargo", ["build", "--release", "-p", "universal-payment-qr-core", "--bin", "upqr-core"], {
  cwd: repoRoot,
  stdio: "inherit",
});

const binName = process.platform === "win32" ? "upqr-core.exe" : "upqr-core";
const bin = path.join(repoRoot, "target", "release", binName);
const schemes = JSON.parse(execFileSync(bin, ["schemes"], { cwd: repoRoot, encoding: "utf8" }));

const sourceCommit = sh("git", ["rev-parse", "HEAD"]);
const now = new Date();
// Local calendar date, not UTC - toISOString() can read as "yesterday" depending on time of day
// and timezone, which is a confusing thing for a human-facing verification date to do.
const generatedAt = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;

// --- 2. identification tier. Derived from `maturity`, which already encodes exactly the ---------
// distinction this needs (see docs/adding-a-scheme.md and each scheme module's own doc comment).
// One documented override: Swiss QR-bill is `experimental` maturity because of a residual, narrow
// uncertainty (the debtor-address block's line count, and the QRR reference's own check digit -
// see CHANGELOG.md) - but the scheme's *identification signature itself* (the SPC v2.x structural
// format plus a real IBAN checksum) is not in doubt, so it gets the same "exact" tier a beta/stable
// scheme would. No other scheme in the current registry needs an override; if a future addition
// does, add it here with the same kind of comment, not silently.
const IDENTIFICATION_OVERRIDES = {
  swiss_qr_bill: "exact",
};

function identificationFor(scheme) {
  if (scheme.id in IDENTIFICATION_OVERRIDES) return IDENTIFICATION_OVERRIDES[scheme.id];
  switch (scheme.maturity) {
    case "stable":
    case "beta":
      return "exact";
    case "community":
      return "generic";
    case "experimental":
    case "deprecated":
      return "heuristic";
    default:
      throw new Error(`${scheme.id}: unhandled maturity "${scheme.maturity}"`);
  }
}

// --- 3. Reshape into the public registry contract (registry/schema/scheme.schema.json). ----------

function toRegistryEntry(scheme) {
  const features = scheme.features ?? [];
  const hasFeature = (needle) => features.some((f) => f.includes(needle));
  return {
    scheme: scheme.id,
    displayName: scheme.displayName,
    country: scheme.countries,
    category: scheme.category,
    standard: scheme.standard,
    parserVersion: scheme.parserVersion,
    maturity: scheme.maturity,
    identification: identificationFor(scheme),
    supported: scheme.staticSupported,
    nonPayment: hasFeature("not-a-payment"),
    lastVerified: generatedAt,
    supports: {
      amount: hasFeature("amount"),
      merchant: hasFeature("merchant"),
      currency: hasFeature("currency"),
      dynamic: scheme.dynamicSupported,
    },
    references: scheme.references,
  };
}

const entries = schemes.map(toRegistryEntry).sort((a, b) => a.scheme.localeCompare(b.scheme));

// --- 4. Validate against the published schema's own required/enum contract before writing --------
// anything, so a bug in this script can't publish a registry that violates its own schema.

const schema = JSON.parse(readFileSync(path.join(repoRoot, "registry/schema/scheme.schema.json"), "utf8"));
function validate(entry) {
  for (const key of schema.required) {
    if (!(key in entry)) throw new Error(`${entry.scheme}: missing required field "${key}"`);
  }
  for (const [key, def] of Object.entries(schema.properties)) {
    if (def.enum && key in entry && !def.enum.includes(entry[key])) {
      throw new Error(`${entry.scheme}: "${key}" = ${JSON.stringify(entry[key])} not in ${JSON.stringify(def.enum)}`);
    }
  }
  for (const key of schema.required) {
    if (key === "supports") {
      for (const subKey of schema.properties.supports.required) {
        if (!(subKey in entry.supports)) throw new Error(`${entry.scheme}: missing supports.${subKey}`);
      }
    }
  }
}
entries.forEach(validate);

const aggregate = {
  registryVersion: "1.0.0",
  generatedAt,
  sourceCommit,
  count: entries.length,
  schemes: entries,
};

// --- 5. Write (or, in --check mode, diff against) registry/schemes/<id>.json + schemes.json. -----

const schemesDir = path.join(repoRoot, "registry", "schemes");
const aggregatePath = path.join(repoRoot, "registry", "schemes.json");

function render(entry) {
  return JSON.stringify(entry, null, 2) + "\n";
}

if (checkOnly) {
  let drifted = false;
  const existingFiles = existsSync(schemesDir) ? new Set(readdirSync(schemesDir)) : new Set();
  for (const entry of entries) {
    const file = `${entry.scheme}.json`;
    existingFiles.delete(file);
    const wantText = render({ ...entry, lastVerified: undefined }); // ignore date-only drift below
    const p = path.join(schemesDir, file);
    if (!existsSync(p)) {
      console.error(`missing: registry/schemes/${file}`);
      drifted = true;
      continue;
    }
    const have = JSON.parse(readFileSync(p, "utf8"));
    const want = { ...entry, lastVerified: have.lastVerified }; // date changes every run; not drift
    if (JSON.stringify(have) !== JSON.stringify(want)) {
      console.error(`out of date: registry/schemes/${file}`);
      drifted = true;
    }
  }
  for (const stale of existingFiles) {
    console.error(`stale (no longer a scheme): registry/schemes/${stale}`);
    drifted = true;
  }
  if (drifted) {
    console.error("\nregistry/ is out of date. Run: node scripts/generate-registry.mjs");
    process.exit(1);
  }
  console.log(`registry/ matches the live core (${entries.length} schemes).`);
  process.exit(0);
}

rmSync(schemesDir, { recursive: true, force: true });
mkdirSync(schemesDir, { recursive: true });
for (const entry of entries) {
  writeFileSync(path.join(schemesDir, `${entry.scheme}.json`), render(entry));
}
writeFileSync(aggregatePath, JSON.stringify(aggregate, null, 2) + "\n");

console.log(`Wrote ${entries.length} files to registry/schemes/ and registry/schemes.json`);
