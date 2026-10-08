import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import test from "node:test";

const versionScript = resolve(".github/scripts/set-version.mjs");
const caskScript = resolve(".github/scripts/homebrew-cask.mjs");

function fixture(t) {
  const directory = mkdtempSync(join(tmpdir(), "time-wise-release-"));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  return directory;
}

function run(script, args, cwd) {
  return spawnSync(process.execPath, [script, ...args], {
    cwd,
    encoding: "utf8",
    env: { ...process.env, GH_REPO: "9renpoto/time-wise" },
  });
}

test("release tags must match all version sources", (t) => {
  const directory = fixture(t);
  mkdirSync(join(directory, "apps/desktop/src-tauri"), { recursive: true });
  for (const path of [
    "Cargo.toml",
    "Cargo.lock",
    "apps/desktop/src-tauri/tauri.conf.json",
  ]) {
    copyFileSync(path, join(directory, path));
  }
  assert.equal(run(versionScript, ["0.2.0"], directory).status, 0);
  assert.equal(
    run(versionScript, ["--check-tag", "v0.2.0"], directory).status,
    0,
  );
  assert.notEqual(
    run(versionScript, ["--check-tag", "v0.3.0"], directory).status,
    0,
  );
  assert.notEqual(run(versionScript, ["--check-tag"], directory).status, 0);
  writeFileSync(
    join(directory, "apps/desktop/src-tauri/tauri.conf.json"),
    '{"version":"0.3.0"}',
  );
  assert.notEqual(
    run(versionScript, ["--check-tag", "v0.2.0"], directory).status,
    0,
  );
});

test("cask checksums and URLs use each architecture's exact release asset", (t) => {
  const directory = fixture(t);
  for (const arch of ["aarch64", "x64"]) {
    writeFileSync(join(directory, `Time Wise_0.2.0_${arch}.dmg`), arch);
  }
  const result = run(caskScript, ["v0.2.0", directory]);
  assert.equal(result.status, 0, result.stderr);
  for (const arch of ["aarch64", "x64"]) {
    assert.ok(
      result.stdout.includes(createHash("sha256").update(arch).digest("hex")),
    );
    assert.ok(result.stdout.includes(`Time%20Wise_0.2.0_${arch}.dmg`));
  }
  assert.ok(result.stdout.includes('app "Time Wise.app"'));
  assert.ok(!result.stdout.includes(":no_check"));
});

test("missing, duplicate, or wrong-version DMGs prevent cask publication", (t) => {
  const directory = fixture(t);
  writeFileSync(join(directory, "Time Wise_0.2.0_aarch64.dmg"), "arm");
  assert.notEqual(run(caskScript, ["v0.2.0", directory]).status, 0);
  writeFileSync(join(directory, "Time Wise_0.2.0_x64.dmg"), "intel");
  assert.notEqual(run(caskScript, ["v0.3.0", directory]).status, 0);
  writeFileSync(join(directory, "Other_0.2.0_x64.dmg"), "duplicate");
  assert.notEqual(run(caskScript, ["v0.2.0", directory]).status, 0);
});
