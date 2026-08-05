#!/usr/bin/env node

import { readFileSync, writeFileSync } from "node:fs";

const VERSION_PATTERN = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/;
const PACKAGE_MANIFESTS = [
  { path: "Cargo.toml", name: "time-wise-ui" },
  { path: "src-tauri/Cargo.toml", name: "time-wise" },
];
const TAURI_CONFIG = "src-tauri/tauri.conf.json";
const CARGO_LOCK = "Cargo.lock";

function readRequiredFile(path) {
  try {
    return readFileSync(path, "utf8");
  } catch (error) {
    if (error?.code === "ENOENT") {
      throw new Error(`${path}: required version source is missing; ensure it is tracked`);
    }
    throw error;
  }
}

function packageVersion(content, path) {
  const lines = content.split(/\r?\n/);
  const packageStart = lines.indexOf("[package]");
  if (packageStart === -1) {
    throw new Error(`${path}: missing [package] section`);
  }

  const packageEnd = lines.findIndex(
    (line, index) => index > packageStart && /^\[.+\]$/.test(line),
  );
  const end = packageEnd === -1 ? lines.length : packageEnd;
  const versionIndex = lines.findIndex(
    (line, index) =>
      index > packageStart && index < end && /^version\s*=\s*"[^"]+"$/.test(line),
  );
  if (versionIndex === -1) {
    throw new Error(`${path}: missing package version`);
  }

  return {
    version: lines[versionIndex].match(/"([^"]+)"/)[1],
    replace(version) {
      lines[versionIndex] = `version = "${version}"`;
      return lines.join("\n");
    },
  };
}

function lockedPackageVersion(content, packageName) {
  const blocks = content.split(/(?=^\[\[package\]\]$)/m);
  const blockIndex = blocks.findIndex((block) => {
    const name = block.match(/^name\s*=\s*"([^"]+)"$/m)?.[1];
    return name === packageName;
  });
  if (blockIndex === -1) {
    throw new Error(`${CARGO_LOCK}: missing package ${packageName}`);
  }

  const version = blocks[blockIndex].match(/^version\s*=\s*"([^"]+)"$/m)?.[1];
  if (!version) {
    throw new Error(`${CARGO_LOCK}: missing version for ${packageName}`);
  }

  return {
    version,
    replace(nextVersion) {
      blocks[blockIndex] = blocks[blockIndex].replace(
        /^version\s*=\s*"[^"]+"$/m,
        `version = "${nextVersion}"`,
      );
      return blocks.join("");
    },
  };
}

function readVersions() {
  const versions = PACKAGE_MANIFESTS.map(({ path, name }) => ({
    source: path,
    value: packageVersion(readRequiredFile(path), path).version,
    name,
  }));
  const tauriConfig = JSON.parse(readRequiredFile(TAURI_CONFIG));
  versions.push({ source: TAURI_CONFIG, value: tauriConfig.version });

  const lock = readRequiredFile(CARGO_LOCK);
  for (const { name } of PACKAGE_MANIFESTS) {
    versions.push({
      source: `${CARGO_LOCK}:${name}`,
      value: lockedPackageVersion(lock, name).version,
    });
  }
  return versions;
}

function checkVersions() {
  const versions = readVersions();
  const expected = versions[0].value;
  const mismatches = versions.filter(({ value }) => value !== expected);
  if (mismatches.length > 0) {
    const details = versions.map(({ source, value }) => `${source}=${value}`).join(", ");
    throw new Error(`version mismatch: ${details}`);
  }
  console.log(`All application versions are ${expected}.`);
}

function updateVersions(version) {
  if (!VERSION_PATTERN.test(version)) {
    throw new Error(`invalid semantic version: ${version}`);
  }

  for (const { path } of PACKAGE_MANIFESTS) {
    const content = readRequiredFile(path);
    writeFileSync(path, packageVersion(content, path).replace(version));
  }

  const configContent = readRequiredFile(TAURI_CONFIG);
  const config = JSON.parse(configContent);
  config.version = version;
  writeFileSync(TAURI_CONFIG, `${JSON.stringify(config, null, 2)}\n`);

  let lockContent = readRequiredFile(CARGO_LOCK);
  for (const { name } of PACKAGE_MANIFESTS) {
    lockContent = lockedPackageVersion(lockContent, name).replace(version);
  }
  writeFileSync(CARGO_LOCK, lockContent);

  checkVersions();
}

const argument = process.argv[2];
if (argument === "--check") {
  checkVersions();
} else if (argument) {
  updateVersions(argument);
} else {
  console.error("Usage: set-version.mjs <semver> | --check");
  process.exitCode = 2;
}
