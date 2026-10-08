# Tag-based releases

Release tags are the publication boundary. Pushing `vMAJOR.MINOR.PATCH` builds
Windows installers and macOS DMGs for Apple Silicon and Intel. Tags must match
the versions in `Cargo.toml`, `Cargo.lock`, and the Tauri configuration. A
prerelease tag such as `v0.2.0-rc.1` publishes a GitHub prerelease and does not
update the stable Homebrew cask. Linux installers are not distributed.

## One-time Homebrew setup

1. Create a public `9renpoto/homebrew-tap` repository with an initial commit on
   its default branch. Homebrew calls this tap `9renpoto/tap`.
2. Create a fine-grained personal access token scoped to that repository with
   **Contents: read and write** permission. Add it to this application's Actions
   secrets as `HOMEBREW_TAP_TOKEN`. The application's `GITHUB_TOKEN` cannot write
   to another repository. Never commit the token.
3. To use another tap, set the Actions variable `HOMEBREW_TAP` to its full
   `owner/homebrew-name` repository name and adjust the documented install
   commands. Branch protection must permit the automation's cask commit.

The workflow updates `Casks/time-wise.rb` on the tap's default branch after a
stable release is published. It selects the matching DMG for each architecture
and records its SHA-256. The generated cask is also attached to GitHub Releases.
The first successful stable release creates the cask; installation is available
only once that update completes.

The workflow currently has no Apple Developer signing or notarization setup.
Homebrew installation does not remove macOS Gatekeeper checks; distribution of
signed and notarized macOS builds requires that setup separately. The cask does
not disable quarantine or remove application history on uninstall.

## Prepare and publish

The scheduled **Bump version (cron)** workflow runs every Tuesday at 03:15 UTC
and calls **Bump version** to open or update a draft release-review PR on the
`release` branch with a minor version increment. **Bump version** can also be
started manually with a major, minor, or patch increment. These workflows prepare
versions and release notes; they do not create tags or publish releases.

Use the Bump PR to review the version changes and release notes, check CI, and
complete pre-release validation. Once the review is complete, mark the PR ready
and merge it into `main`. Then tag the merged version commit to publish.

To prepare the version changes locally instead:

```bash
node .github/scripts/set-version.mjs 0.2.0
node .github/scripts/set-version.mjs --check-tag v0.2.0
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

Once the version commit is on `main`, tag that commit and push the tag:

```bash
git switch main
git pull --ff-only origin main
git tag -a v0.2.0 -m 'Time Wise v0.2.0'
git push origin v0.2.0
```

The **Release** workflow validates versions, creates a draft with generated
release notes, uploads all three platform builds, generates the cask, and
publishes only after all builds succeed. Stable releases become the latest
release. Prereleases do not. Release runs are serialized to prevent simultaneous
tap updates.

## Verify or recover

Check that the release contains Windows `.exe` and `.msi` installers, both
architecture-specific `.dmg` files, and `time-wise.rb`. Check the Homebrew job
and the cask commit in the tap, then verify installation on a Mac:

```bash
brew tap 9renpoto/tap
brew update
brew install --cask 9renpoto/tap/time-wise
```

If a build fails, the release stays in draft. Re-run the failed jobs, or run
**Release** manually with the existing tag to retry a draft. An already
published release is rejected by preparation to avoid overwriting public
assets. If only the tap update fails, fix the token or tap permissions and
re-run the failed Homebrew job in the original workflow run. Do not move a
published tag; prepare a new version for application fixes.
