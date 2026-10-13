#!/usr/bin/env node

import { createHash } from "node:crypto";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

const [tag, directory] = process.argv.slice(2);
if (!/^v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(tag ?? "") || !directory) {
  throw new Error("Usage: homebrew-cask.mjs <vSemVer> <asset-directory>");
}
const version = tag.slice(1);
const repository = process.env.GH_REPO ?? "9renpoto/time-wise";
if (!/^[\w.-]+\/[\w.-]+$/.test(repository)) {
  throw new Error("Invalid GitHub repository");
}
const files = readdirSync(directory);
const assets = ["aarch64", "x64"].map((arch) => {
  const matches = files.filter((file) =>
    file.endsWith(`_${version}_${arch}.dmg`),
  );
  if (matches.length !== 1) {
    throw new Error(
      `Expected one ${arch} DMG for ${version}, found ${matches.length}`,
    );
  }
  const file = matches[0];
  return {
    file,
    sha256: createHash("sha256")
      .update(readFileSync(join(directory, file)))
      .digest("hex"),
  };
});

console.log(`cask "time-wise" do
  version "${version}"

  on_arm do
    sha256 "${assets[0].sha256}"
    url "https://github.com/${repository}/releases/download/v#{version}/${encodeURIComponent(assets[0].file)}"
  end

  on_intel do
    sha256 "${assets[1].sha256}"
    url "https://github.com/${repository}/releases/download/v#{version}/${encodeURIComponent(assets[1].file)}"
  end

  name "Time Wise"
  desc "Track focused-application time and store usage history on your device"
  homepage "https://github.com/${repository}"

  depends_on :macos

  app "Time Wise.app"
end`);
