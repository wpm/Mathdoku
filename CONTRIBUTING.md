# Contributing to Mathdoku

Thanks for your interest in the project.

By submitting a contribution you agree to license it under the project's
[Apache-2.0 license](LICENSE).

## Commit messages: Conventional Commits

This repository uses [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/).
Release automation ([release-plz](https://release-plz.dev/), see
[`RELEASING.md`](RELEASING.md)) derives version bumps and changelog entries
for the `mathdoku` crate from commit messages, so the format is a requirement,
not a style preference.

Format: `<type>[optional scope][!]: <description>`

- `feat:` — a new feature (minor version bump)
- `fix:` — a bug fix (patch version bump)
- A `!` after the type/scope, or a `BREAKING CHANGE:` footer, marks a breaking
  change (major version bump; minor while the crate is pre-1.0)
- Other useful types: `docs:`, `test:`, `refactor:`, `perf:`, `chore:`, `ci:`,
  `build:`

Examples:

```
feat(solver): add collinear distinctness memo
fix: reject cages with duplicate cells
feat!: replace Grid::solutions iterator with a Solutions struct
```

Only commits that touch `mathdoku/` affect that crate's release; commits
scoped to the designer apps still benefit from the convention for history
readability.

## Development workflow

- Enable the shared git hooks so your commits run the same checks as CI:
  `git config core.hooksPath .githooks`.
- The workspace enforces a strict lint policy (`clippy::all`, `pedantic`,
  `nursery`, plus denied `unwrap`/`expect`/`panic`/`todo` paths). The
  `mathdoku` crate has not yet opted into the full workspace lints pending an
  error-handling cleanup; see issue #59.
- Significant design decisions are recorded as ADRs under `adr/`. Add a new one
  when proposing an architecturally significant change.
- Note user-facing library changes in [`mathdoku/CHANGELOG.md`] under the
  `[Unreleased]` section. (release-plz also generates entries from commit
  messages; hand-written entries take precedence when both describe a change.)

Build and test commands are documented in the
[README](README.md#building-and-testing); the authoritative command set lives
in [`.github/workflows/ci.yml`] and the shared [`.githooks/pre-commit`] hook.

## PR preview deploys

Every pull request gets a WASM build of the Designer deployed to the
`gh-pages` branch under `/pr-N/` (where `N` is the PR number), served at
`https://wpm.github.io/Mathdoku/pr-N/`. You don't need to do anything to get
one:

- A sticky comment on the PR links to the preview and updates on each push
  (via [rossjrw/pr-preview-action]).
- When the PR closes, the preview is removed, and its entry in the
  repository's Environments sidebar is deactivated and deleted.
- PRs from forks skip the preview deploy — GitHub gives their workflow runs a
  read-only `GITHUB_TOKEN` — but the bundle-size gate in
  [`.github/workflows/ci.yml`] (the part that actually gates merge) still
  runs for them.

The workflow itself is [`.github/workflows/pr-preview.yml`].

[rossjrw/pr-preview-action]: https://github.com/rossjrw/pr-preview-action
[`.github/workflows/pr-preview.yml`]: .github/workflows/pr-preview.yml

[`mathdoku/CHANGELOG.md`]: mathdoku/CHANGELOG.md
[`.github/workflows/ci.yml`]: .github/workflows/ci.yml
[`.githooks/pre-commit`]: .githooks/pre-commit
