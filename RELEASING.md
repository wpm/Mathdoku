# Releasing `mathdoku`

Releases of the `mathdoku` crate are automated with
[release-plz](https://release-plz.dev/) (configuration in
[`release-plz.toml`](release-plz.toml), workflow in
[`.github/workflows/release.yml`](.github/workflows/release.yml)). The
designer application crates are not released to crates.io; the desktop
Designer app is released separately as a downloadable, auto-updating
bundle — see [Releasing the Designer desktop app](#releasing-the-designer-desktop-app)
below.

## How the automation works

1. On every push to `main`, `release-plz release-pr` opens (or updates) a
   **release PR** containing the version bump in `mathdoku/Cargo.toml` and the
   generated changelog entries in `mathdoku/CHANGELOG.md`. Version bumps and
   changelog content are derived from
   [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/)
   messages — see [`CONTRIBUTING.md`](CONTRIBUTING.md).
2. Merging the release PR pushes the new version to `main`, where
   `release-plz release` tags it (`mathdoku-v<version>`), publishes `mathdoku`
   to crates.io, and creates the GitHub release.
3. Publishing authenticates via
   [crates.io Trusted Publishing](https://crates.io/docs/trusted-publishing)
   (GitHub OIDC). release-plz obtains a short-lived token at publish time;
   there is **no `CARGO_REGISTRY_TOKEN` repository secret**, and no API token
   to rotate.

## Human steps at each release

The automation handles versioning, changelog, tagging, and publishing. What
remains:

1. **MSRV check (N−2 policy).** The crate's MSRV lags the latest stable Rust
   by two releases. Before merging a release PR, check the
   [latest stable release](https://github.com/rust-lang/rust/blob/master/RELEASES.md)
   and, if `rust-version` in `mathdoku/Cargo.toml` is more than two releases
   behind, bump it (a regular PR, before the release PR merges) along with
   the README Prerequisites section. An MSRV bump is at least a minor version
   bump for the crate.
2. **Review the release PR.** Sanity-check the proposed version against
   semver (release-plz infers it from commit types; a mis-typed commit means a
   wrong bump), and edit the generated changelog entries for readability —
   hand-edits to the PR survive publication.
3. **Check CI is green on the release PR.** PRs opened with the default
   `GITHUB_TOKEN` do not trigger CI workflows (a GitHub limitation). Close
   and reopen the release PR to trigger CI, or push an empty commit to its
   branch.
4. **Merge.** The `release` job does the rest. Verify afterward:
   - the new version appears on [crates.io](https://crates.io/crates/mathdoku),
   - the [docs.rs build](https://docs.rs/mathdoku) succeeds,
   - the `mathdoku-v<version>` tag and GitHub release exist.

## Releasing the Designer desktop app

The desktop Designer is released independently of the `mathdoku` crate. Its
version lives in
[`apps/designer/src-tauri/tauri.conf.json`](apps/designer/src-tauri/tauri.conf.json)
(and the matching `Cargo.toml`), is **not** managed by release-plz, and is
unrelated to the `mathdoku-v<version>` crate tags.

A push of a tag matching `designer-v*` triggers
[`.github/workflows/release-designer.yml`](.github/workflows/release-designer.yml),
which builds the app for macOS (Apple Silicon + Intel), Windows, and Linux via
[`tauri-action`](https://github.com/tauri-apps/tauri-action), signs each bundle
and the auto-updater manifest, and creates a **draft** GitHub release carrying
the installers and a signed `latest.json`.

### Prerequisites (one-time)

Two sets of repository secrets must be configured (Settings → Secrets and
variables → Actions):

1. `TAURI_SIGNING_PRIVATE_KEY` (see [Signing key](#signing-key) below).
   Without it, the build still produces installers but `tauri-action` cannot
   sign `latest.json`, and the auto-updater will reject the release.
2. The six `APPLE_*` secrets (see
   [Apple code-signing secrets](#apple-code-signing-secrets) below). Without
   them, macOS bundles build unsigned and un-notarized; Gatekeeper will warn
   or block users who download them.

### Steps to cut a Designer release

1. **Bump the app version.** Set the new version in
   `apps/designer/src-tauri/tauri.conf.json` (and `Cargo.toml`), commit, and
   merge to `main`. Use the same version string you will tag with.
2. **Tag and push.** Tag the merged commit `designer-v<version>` (matching the
   `tauri.conf.json` version) and push the tag:

   ```sh
   git tag designer-v<version>
   git push origin designer-v<version>
   ```

   The tag — not a branch push — is what triggers the workflow. (You can also
   run it manually from the Actions tab via `workflow_dispatch`, in which case
   the version comes from `tauri.conf.json`.)
3. **Wait for the matrix build.** All four platform jobs must succeed; each
   uploads its installer(s) to the same draft release.
4. **On the first release, confirm the manifest filename.** The updater
   endpoint in `tauri.conf.json` points at `latest.json`, which is
   `tauri-action`'s default manifest name. Check that the draft release
   actually has a `latest.json` asset attached. If a future `tauri-action`
   version names it differently, update either the workflow or the
   `plugins.updater.endpoints` URL so the two agree — otherwise the updater
   fetches a 404.
5. **Publish the draft release.** The workflow leaves the release as a draft so
   you can sanity-check the attached bundles first. **The auto-updater endpoint
   (`releases/latest/download/latest.json`) only resolves once the release is
   published** — a draft is invisible to it. Edit the release notes, then click
   *Publish release*.
6. **Verify auto-update.** After publishing, confirm a previously installed
   copy detects and applies the update. (The very first release has nothing to
   update *from*; the round-trip is only testable from the second release
   onward.)

### Signing key

`tauri-action` signs the bundles and `latest.json` with the minisign private
key in the `TAURI_SIGNING_PRIVATE_KEY` repository secret (generated with
`cargo tauri signer generate`, no password). Its public half is embedded in
`tauri.conf.json` under `plugins.updater.pubkey`. **The private key is never
committed and cannot be recovered** — if it is lost, existing installs can no
longer verify updates and every user must reinstall from a freshly keyed
download.

#### Rotating the signing key

Rotate only when necessary — a lost private key, or a suspected compromise.
**Rotation breaks the update chain across the boundary.** An installed app
verifies each downloaded update against the `pubkey` baked into *that
installed binary*. A release signed with a new key cannot be verified by any
copy still carrying the old `pubkey`, so every existing install must do one
manual reinstall to pick up the new key; auto-update resumes from there. There
is no way to avoid this — it is the security property working as intended.

1. **Generate a new keypair.** From anywhere:

   ```sh
   cargo tauri signer generate -w mathdoku-designer.key -p ""
   ```

   This writes the private key to `mathdoku-designer.key` and the public key to
   `mathdoku-designer.key.pub`. Keep both out of the repository.
2. **Replace the repository secret.** Set `TAURI_SIGNING_PRIVATE_KEY` (repo
   Settings → Secrets and variables → Actions) to the **full contents** of
   `mathdoku-designer.key`. The key has no password, so
   `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` stays empty in the workflow.
3. **Embed the new public key.** Copy the contents of
   `mathdoku-designer.key.pub` into `plugins.updater.pubkey` in
   `apps/designer/src-tauri/tauri.conf.json`, commit, and merge to `main`. This
   is the change that, once shipped in a build, lets future installs trust the
   new key.
4. **Cut a release** following the steps above. The build signs with the new
   private key and ships the new `pubkey`.
5. **Tell existing users to reinstall.** Note in the release announcement /
   download page that users on any prior version must download and reinstall
   once; their auto-updates were signed with the retired key and will not
   apply. (Skippable only if there is no meaningful installed base yet.)
6. **Destroy the old private key.** Once the rotated release is published,
   securely delete every copy of the previous `mathdoku-designer.key` so a
   leaked old key can never sign anything again.

### Apple code-signing secrets

macOS bundles are Developer ID-signed and notarized in CI. Six repository
secrets drive this; `tauri-action` consumes the first three, and both
`tauri-action` and the workflow's DMG-staple step use the rest:

| Secret | Value |
| --- | --- |
| `APPLE_CERTIFICATE` | base64 of the Developer ID Application `.p12` export (`base64 -i cert.p12`) |
| `APPLE_CERTIFICATE_PASSWORD` | password chosen when exporting the `.p12` |
| `APPLE_SIGNING_IDENTITY` | full identity string, e.g. `Developer ID Application: <name> (<team id>)` — must match the certificate exactly |
| `APPLE_ID` | Apple ID email of the developer account |
| `APPLE_PASSWORD` | an [app-specific password](https://account.apple.com) for that Apple ID (not the account password) |
| `APPLE_TEAM_ID` | 10-character team ID (also visible in the identity string) |

The signing identity is deliberately **not** set in `tauri.conf.json`: keeping
it in a secret leaves the repo free of personal identifiers and lets local dev
builds run unsigned. No entitlements file is needed — notarization passes with
the bundler's default hardened runtime.

Maintenance notes:

- **The `.p12` is irreplaceable.** Apple does not re-issue private keys, and
  an account is limited to five Developer ID Application certificates total.
  Keep the `.p12` export backed up securely.
- **App-specific passwords** can be revoked and regenerated at
  [account.apple.com](https://account.apple.com) at any time without touching
  the certificate; update `APPLE_PASSWORD` afterward.
- **Certificate expiry** (5 years): generate a new certificate via
  Keychain Access CSR + [developer.apple.com](https://developer.apple.com/account/resources/certificates/add),
  export a new `.p12`, and update `APPLE_CERTIFICATE`,
  `APPLE_CERTIFICATE_PASSWORD`, and `APPLE_SIGNING_IDENTITY`. Already-shipped
  releases keep working; notarization tickets and stapled signatures outlive
  the certificate.

## One-time bootstrap (already-planned, maintainer-only)

crates.io requires a crate's **first** publish to use a real API token;
Trusted Publishing applies from the second publish onward. The plan is to
publish a `0.0.0` placeholder to reserve the name and unlock Trusted
Publishing:

1. Verify `mathdoku` is still free on
   [crates.io](https://crates.io/search?q=mathdoku).
2. Create an API token with the `publish-new` scope at
   <https://crates.io/settings/tokens>.
3. Temporarily set `version = "0.0.0"` in `mathdoku/Cargo.toml`, run
   `cargo publish -p mathdoku --dry-run`, then `cargo publish -p mathdoku`.
   Revert the version afterward — do not commit `0.0.0`.
4. On crates.io: `mathdoku` crate → Settings → Trusted Publishing → add a
   GitHub publisher with repository `wpm/Mathdoku` and workflow filename
   `release.yml`.
5. Revoke the API token — it is never needed again.
