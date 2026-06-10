# Mathdoku

[![CI](https://github.com/wpm/Mathdoku/actions/workflows/ci.yml/badge.svg)](https://github.com/wpm/Mathdoku/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/wpm/Mathdoku/branch/main/graph/badge.svg)](https://codecov.io/gh/wpm/Mathdoku)

A Rust workspace for generating, solving, and designing Mathdoku puzzles.

## Workspace layout

| Path | Crate | Description |
|------|-------|-------------|
| `mathdoku/` | `mathdoku` | Core library: puzzle representation, constraint propagation, solver, and generator. |
| `apps/designer/` | `mathdoku-designer-ui` | Leptos/WASM UI for the desktop designer. |
| `apps/designer/core/` | `mathdoku-designer-core` | Platform-independent designer logic. |
| `apps/designer/src-tauri/` | `mathdoku-designer-tauri` | Tauri desktop shell. |
| `adr/` | — | Architecture Decision Records. |

Only `mathdoku` is intended for publication to crates.io. The designer crates
are marked `publish = false`.

## Prerequisites

- A stable Rust toolchain. `mathdoku` sets `rust-version = "1.94"`; match or
  exceed it. (Policy: the MSRV lags the latest stable release by two versions;
  see [`RELEASING.md`](RELEASING.md).)
- For the designer: the `wasm32-unknown-unknown` target, [Trunk], and the
  [Tauri CLI] for the desktop shell.
- For the end-to-end tests: Node 22+ and the Playwright Chromium browser
  (`apps/designer/e2e/`).

[Trunk]: https://trunkrs.dev/
[Tauri CLI]: https://tauri.app/

See the crate's [README](mathdoku/README.md) and the
[API documentation](https://docs.rs/mathdoku) for the full surface, including
programmatic puzzle construction with `Puzzle::new` / `Puzzle::insert`.

## Building and testing

The authoritative command set lives in [`.github/workflows/ci.yml`] and the
shared [`.githooks/pre-commit`] hook. The essentials:

```sh
# Core library
cargo build -p mathdoku
cargo test --lib -p mathdoku
cargo doc --no-deps -p mathdoku

# Designer (run from its directory)
cd apps/designer && cargo test
```

Some library tests are marked `#[ignore]` because they are slow; run them with
`cargo test --lib -p mathdoku -- --include-ignored`.

[`.github/workflows/ci.yml`]: .github/workflows/ci.yml
[`.githooks/pre-commit`]: .githooks/pre-commit

## Running the designer

- **Web preview** (client-side rendering): from `apps/designer/`, run
  `trunk serve` and open <http://localhost:1420>.
- **Desktop app**: with the [Tauri CLI] installed, run `cargo tauri dev` from
  `apps/designer/src-tauri/`.

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the development workflow, commit
message conventions, and what to expect when submitting changes.

## License

Licensed under the [Apache License, Version 2.0](LICENSE).
