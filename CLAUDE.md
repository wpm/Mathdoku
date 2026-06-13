# CLAUDE.md

Guidance for AI coding agents (Claude Code web and local) working in this
repository.

## Commit and PR conventions

PRs are squash-merged. The PR title becomes the commit message on `main`,
which release-plz parses to derive version bumps and changelog entries for
the published `mathdoku` crate. Consequently:

- **PR titles MUST follow [Conventional Commits](https://www.conventionalcommits.org/):**
  `type(optional-scope): description`. This is enforced by
  `.github/workflows/pr-title.yml`; a non-conforming title fails the PR.
- Types and their release effect on `mathdoku`: `fix` → patch bump, `feat` →
  minor bump, `!` suffix or `BREAKING CHANGE` footer → breaking bump
  (compressed semver below 1.0). Also available: `docs`, `refactor`, `perf`,
  `test`, `build`, `ci`, `chore`, `revert`.
- Branch commits are squashed away, so their messages are not enforced —
  but use the conventional format there too; it keeps history legible and
  PR titles honest.
- When creating a PR for an issue, ensure that it when it closes, the issue
  closes as well.
- Never use the word "KenKen" anywhere (trademark). The project and genre
  name is "Mathdoku".
