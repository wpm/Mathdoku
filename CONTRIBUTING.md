# Contributing to Mathdoku

Thanks for your interest in the project.

## Expectations

This is a personal project. Forks are welcome, and pull requests will be
considered, but there is no commitment to review or merge external
contributions on any timeline — or at all. If you need a change for your own
use, forking is often the fastest path. For anything beyond a small fix, open
an issue first so effort isn't wasted on a change that won't land.

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

[`mathdoku/CHANGELOG.md`]: mathdoku/CHANGELOG.md
[`.github/workflows/ci.yml`]: .github/workflows/ci.yml
[`.githooks/pre-commit`]: .githooks/pre-commit
