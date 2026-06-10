# Releasing `mathdoku`

Releases of the `mathdoku` crate are automated with
[release-plz](https://release-plz.dev/) (configuration in
[`release-plz.toml`](release-plz.toml), workflow in
[`.github/workflows/release.yml`](.github/workflows/release.yml)). The
designer application crates are not released to crates.io.

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
