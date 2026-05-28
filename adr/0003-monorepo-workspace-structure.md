# ADR-0003: Monorepo workspace structure

**Status:** Accepted
**Date:** 2026-05-28
**Deciders:** W.P. McNeill (Mathdoku owner)

## Context

The repository hosts a Rust library (`mathdoku`) and a desktop application (`mathdoku-designer`), with more applications anticipated — a research CLI, possibly a server-side variant, possibly puzzle-import tools. The owner's goals for the layout, stated explicitly, are: a monorepo that is easy to work in; the ability to version and release the library independently of any application; the ability to add further applications that depend on the library without re-litigating structure each time; and library-only publishing to crates.io, with applications never published.

The repository currently nests workspaces. The root `Cargo.toml` declares a workspace with `mathdoku` as its sole member. `mathdoku-designer/Cargo.toml` declares a *second* workspace whose members are the designer UI crate (the manifest itself, package `mathdoku-designer-ui`), `core` (`mathdoku-designer-core`), and `src-tauri` (`mathdoku-designer`, the Tauri shell binary). Each workspace owns its own `Cargo.lock` and `target/`. The two lockfiles drift independently. `cargo test --workspace` from the root never reaches the designer.

The strict clippy policy — `deny(clippy::all, pedantic, nursery)` plus `deny(unwrap_used, expect_used, panic, todo, unimplemented, dbg_macro, print_stdout, print_stderr)` — currently lives in the designer's nested `[workspace.lints]`. `mathdoku` opts in to no such policy and declares only `[lints.rust] unused_results = "warn"` directly.

The decision is therefore the workspace shape that ships at 0.1.0 and the lint policy that goes with it.

## Decision

A single flat Cargo workspace at the repository root, with applications living under `apps/` and the library staying at root.

```
Mathdoku/
├── Cargo.toml                  # workspace root
├── Cargo.lock                  # single lockfile
├── target/
├── mathdoku/                   # publishable library
└── apps/
    └── designer/
        ├── Cargo.toml          # mathdoku-designer-ui (Leptos/WASM frontend)
        ├── core/Cargo.toml     # mathdoku-designer-core
        └── src-tauri/Cargo.toml  # mathdoku-designer-tauri (Tauri shell binary)
```

Workspace members: `["mathdoku", "apps/designer", "apps/designer/core", "apps/designer/src-tauri"]`. Future applications go under `apps/` as siblings of `designer/`.

Multi-crate units — applications, library families, anything that fans out into more than one crate — follow a parallel-naming convention: every crate is named `<unit>-<role>`. The Designer's three crates are therefore `mathdoku-designer-ui` (Leptos/WASM frontend), `mathdoku-designer-core` (command bodies and `AppState`), and `mathdoku-designer-tauri` (Tauri shell binary). No crate takes the bare unit name. Leaving `mathdoku-designer` unclaimed by any package keeps each crate's role unambiguous from its name alone — a contributor grepping `mathdoku-designer-` lands on the role they want, and the umbrella name refers unambiguously to the directory, not a specific component.

Application bundle identifiers follow the parallel scheme `com.wpmcneill.<project>.<app>`. The bare project segment `com.wpmcneill.mathdoku` is reserved as a namespace and never assigned to a concrete app. The Designer is `com.wpmcneill.mathdoku.designer`; future GUI apps follow the same pattern (`com.wpmcneill.mathdoku.research`, `com.wpmcneill.mathdoku.solver`, …). Dots rather than hyphens — i.e., not `com.wpmcneill.mathdoku-designer` — match the Apple-tooling convention: app groups for shared keychain entries or files between sibling apps root naturally at `group.com.wpmcneill.mathdoku`, code-signing trust evaluation keys off the dotted prefix, and `LSApplicationCategory`-style introspection treats the hierarchy as load-bearing. The library `mathdoku` on crates.io has no bundle identifier — those are app-only. Changing a shipped app's bundle ID later means a fresh install on every user's machine, so the scheme is locked at 0.1.0.

The root `[workspace.package]` carries `edition`, `authors = ["W.P. McNeill"]`, `repository = "https://github.com/wpm/Mathdoku"`, and `license = "MIT"`. It does **not** carry `version`. Each crate declares its own `version` in `[package]`, so `mathdoku 0.1.5` and `mathdoku-designer 0.4.2` are released independently. `mathdoku` is publishable; every application crate sets `publish = false`.

The strict clippy/rust lint policy moves into the root `[workspace.lints]`. Every crate opts in via `[lints] workspace = true` — including `mathdoku`. The opt-in is uniform and explicit; the per-crate `[lints]` block names the policy without restating it.

`mathdoku-designer/` becomes a synonym for `apps/designer/` after the move. Path dependencies update accordingly: the UI crate's `mathdoku = { path = "../mathdoku" }` becomes `path = "../../mathdoku"`; sub-crates' `path = "../../mathdoku"` becomes `path = "../../../mathdoku"`. Inter-designer path deps (`mathdoku-designer-core = { path = "core" }`) are unchanged by depth.

## Options Considered

### Option A: Single flat workspace, `apps/` subdir, library at root — *chosen*

| Dimension | Assessment |
|-----------|------------|
| Monorepo ergonomics | High — one `Cargo.lock`, one `target/`, one `cargo test --workspace` |
| Independent versioning | Native — per-crate `[package].version`, no shared field |
| Future apps | Drop into `apps/`, no decisions to relitigate |
| Library-only crates.io | `publish = false` on apps, `mathdoku` publishable |
| Strict lints | Uniform via `[workspace.lints]`, explicit opt-in per crate |
| Restructuring lift | Moderate — directory move + path-dep adjustments + lint-policy cleanup in `mathdoku` |

**Pros:** Aligns directly with every stated goal. Single source of dependency resolution removes a class of "works in one workspace, not the other" bugs. The `apps/` directory communicates intent — a contributor opening the repo immediately sees what is published and what is shipped. `release-plz` and `cargo publish` honor `publish = false`, so library-only crates.io publishing comes for free.

**Cons:** `mathdoku/` at the root sits asymmetrically next to `apps/`. Tolerable while there is one library; if a second library appears, introduce `crates/` (or `lib/`) and revisit. Applying the strict lint policy to `mathdoku` requires a cleanup pass — at least one production `.unwrap()` in `csp.rs:200` violates `unwrap_used`, and `expect_used` and `panic` are also denied, so the fix is real error handling rather than a regex pass. Tracked as separate work; the workspace flattening lands first, `mathdoku` opts in to `[lints] workspace = true` once the cleanup ships.

### Option B: Single flat workspace, library under `crates/` or `lib/`

**Pros:** Symmetric: `crates/mathdoku/` next to `apps/designer/`. Adding a second library is trivial and uniform — no special case for the first.

**Cons:** Solves a problem that does not exist yet (one library). Doc links, file URLs, and any external tooling that references `mathdoku/src/...` break for no current benefit. Easier to introduce `crates/` later when a second library actually appears than to predict the right umbrella now.

### Option C: Nested workspaces — *status quo*

**Pros:** No restructuring. The designer's strict lint policy stays scoped to the designer without an explicit opt-out in `mathdoku`.

**Cons:** Two `Cargo.lock` files drift independently. Two `target/` directories slow iteration and waste disk. `cargo test --workspace`, `cargo fmt --all`, and `cargo clippy --workspace` from the root all miss the designer. Independent versioning works in a flat workspace too — nesting earns nothing for it. Adding a third crate forces a meta-decision (which workspace does it join?) and risks a third nested workspace.

### Option D: Separate repositories per crate

**Pros:** Strongest isolation. Each repo has its own CI surface, its own release cadence, its own contributor model.

**Cons:** Directly contradicts the monorepo goal. Cross-crate refactors become coordinated PRs across repos. `path = "../mathdoku"` dependencies are replaced by either git submodules or pre-publish coordination, both painful before any 0.1.0 exists. The argument for separate repos is usually that crates have genuinely different audiences or governance — here they don't.

## Consequences

The directory move from `mathdoku-designer/` to `apps/designer/` shifts every path dependency by one segment. The nested `[workspace]` block in `mathdoku-designer/Cargo.toml` (now `apps/designer/Cargo.toml`) is removed; its `[workspace.lints]` content is hoisted to the root.

`mathdoku-designer/src-tauri/Cargo.toml` carries `authors = ["W.P. McNeill"]` literally; after the move it switches to `authors.workspace = true`. The Tauri shell crate also renames from package `mathdoku-designer` to `mathdoku-designer-tauri` to satisfy the parallel-naming convention; the cascade reaches the `[lib]` name (and therefore `main.rs`'s `extern` reference) and the Tauri binary name inside the bundle (which can be preserved at `mathdoku-designer` via `mainBinaryName` in `tauri.conf.json` if a short Linux command name matters).

`mathdoku`'s production code is not yet free of `.unwrap()`, `.expect()`, and bare `panic!` — the strict policy applies once those are converted to fallible interfaces or, where panicking truly cannot occur, an explicit `#[allow]` with reason. Mechanical conversion is not the right move; the cleanup involves API-level choices about error types that overlap with Phase 2 (public API surface audit).

Future libraries beyond `mathdoku` are an open question — none today, but `mathdoku-solver` or `mathdoku-storage` are plausible. The first such addition is the trigger to introduce `crates/` and move `mathdoku/` into it. Doing so before that point would be premature.

Tooling that hardcodes the `mathdoku-designer/` path — CI workflows, Trunk config, scripts — needs the path update as part of the move. Worth a `grep -r mathdoku-designer` pass to catch them all.
