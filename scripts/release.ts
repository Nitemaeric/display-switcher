#!/usr/bin/env bun
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

const root = join(import.meta.dir, "..");
const bumpType = process.argv[2];
const shouldPush = process.argv.includes("--push");

function usage(): never {
  console.error("Usage: bun run release <patch|minor|major> [--push]");
  process.exit(1);
}

if (!bumpType || !["patch", "minor", "major"].includes(bumpType)) {
  usage();
}

function parseVersion(version: string): [number, number, number] {
  const match = version.match(/^(\d+)\.(\d+)\.(\d+)$/);
  if (!match) {
    throw new Error(`Invalid semver: ${version}`);
  }
  return [Number(match[1]), Number(match[2]), Number(match[3])];
}

function bumpVersion(version: string, type: string): string {
  let [major, minor, patch] = parseVersion(version);

  if (type === "major") {
    major += 1;
    minor = 0;
    patch = 0;
  } else if (type === "minor") {
    minor += 1;
    patch = 0;
  } else {
    patch += 1;
  }

  return `${major}.${minor}.${patch}`;
}

function setPackageJson(version: string) {
  const path = join(root, "package.json");
  const pkg = JSON.parse(readFileSync(path, "utf8"));
  pkg.version = version;
  writeFileSync(path, `${JSON.stringify(pkg, null, 2)}\n`);
}

function setCargoToml(version: string) {
  const path = join(root, "src-tauri", "Cargo.toml");
  const content = readFileSync(path, "utf8").replace(
    /^version = ".*"$/m,
    `version = "${version}"`,
  );
  writeFileSync(path, content);
}

function setTauriConf(version: string) {
  const path = join(root, "src-tauri", "tauri.conf.json");
  const conf = JSON.parse(readFileSync(path, "utf8"));
  conf.version = version;
  writeFileSync(path, `${JSON.stringify(conf, null, 2)}\n`);
}

function git(...args: string[]) {
  const result = spawnSync("git", args, {
    cwd: root,
    stdio: "inherit",
    shell: process.platform === "win32",
  });

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

function currentBranch(): string {
  const result = spawnSync("git", ["rev-parse", "--abbrev-ref", "HEAD"], {
    cwd: root,
    encoding: "utf8",
    shell: process.platform === "win32",
  });

  if (result.status !== 0 || !result.stdout?.trim()) {
    throw new Error("Could not determine current git branch");
  }

  return result.stdout.trim();
}

const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
const currentVersion = pkg.version as string;
const nextVersion = bumpVersion(currentVersion, bumpType);
const tag = `v${nextVersion}`;

const tagExists = spawnSync("git", ["rev-parse", tag], {
  cwd: root,
  stdio: "ignore",
  shell: process.platform === "win32",
});

if (tagExists.status === 0) {
  console.error(`Tag ${tag} already exists`);
  process.exit(1);
}

console.log(`Bumping ${currentVersion} -> ${nextVersion}`);

setPackageJson(nextVersion);
setCargoToml(nextVersion);
setTauriConf(nextVersion);

git("add", "package.json", "src-tauri/Cargo.toml", "src-tauri/tauri.conf.json");
git("commit", "-m", `Release ${tag}`);
git("tag", "-a", tag, "-m", `Release ${tag}`);

console.log(`Created tag ${tag}`);

if (shouldPush) {
  const branch = currentBranch();
  git("push", "origin", branch);
  git("push", "origin", tag);
  console.log(`Pushed ${branch} and ${tag} to origin`);
} else {
  console.log(`Push when ready: git push origin ${currentBranch()} && git push origin ${tag}`);
}