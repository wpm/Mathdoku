# mathdoku

[![crates.io](https://img.shields.io/crates/v/mathdoku.svg)](https://crates.io/crates/mathdoku)
[![docs.rs](https://img.shields.io/docsrs/mathdoku)](https://docs.rs/mathdoku)
[![CI](https://github.com/wpm/Mathdoku/actions/workflows/ci.yml/badge.svg)](https://github.com/wpm/Mathdoku/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/wpm/Mathdoku/branch/main/graph/badge.svg?flag=mathdoku)](https://app.codecov.io/gh/wpm/Mathdoku?flags%5B0%5D=mathdoku)

A library for generating and solving Mathdoku puzzles.

Mathdoku is an arithmetic logic puzzle played on an n×n grid. The grid is
divided into *cages*, each labeled with a target number and an arithmetic
operation. The goal is to fill the grid with digits 1 through n such that no
digit repeats in any row or column, and the digits in each cage produce the
target value using the given operation.

## Quick start

```toml
[dependencies]
mathdoku = "0.1"
rand = "0.10"
```

Generate a random puzzle and enumerate its solutions:

```rust
use mathdoku::{Error, generate};

fn main() -> Result<(), Error> {
    let mut rng = rand::rng();
    let puzzle = generate(4, &mut rng)?; // random 4×4 puzzle

    for solution in puzzle.solutions() {
        let solution = solution?; // a fully determined `Puzzle`
        println!("{solution}");
    }
    Ok(())
}
```

See the [API documentation](https://docs.rs/mathdoku) for the full surface,
including programmatic puzzle construction with `Puzzle::new` /
`Puzzle::insert`, cell inspection with `Puzzle::get`, and solution-count
queries.

## What the library provides

- **Generation**: `generate(n, rng)` builds a random puzzle from a random
  Latin square and a random cage tiling; `generate_with` exposes the operation
  policy and cage-size distribution for callers who want control over puzzle
  character.
- **Solving**: a constraint-propagation solver behind
  `Puzzle::solutions()`, an iterator that yields each solution as a solved
  `Puzzle`. Take one element to solve, two to test uniqueness.
- **Construction**: build puzzles cage by cage with `Puzzle::insert`;
  infeasible cages are rejected at insertion time.
- **Serialization**: puzzles serialize with [serde] to a stable JSON
  representation of digits and candidates.

All generation and solving entry points take `&mut impl Rng` rather than
seeding internally, so results are reproducible with a seeded generator such
as [rand_chacha].

[serde]: https://docs.rs/serde
[rand_chacha]: https://docs.rs/rand_chacha


## Mathdoku Designer

This crate is the engine behind [Mathdoku Designer], a desktop application
for designing puzzles by hand with live solvability checking. The Designer
consumes the same public API documented here.

[Mathdoku Designer]: https://wpm.github.io/Mathdoku/

## License

Licensed under the [Apache License, Version 2.0](https://github.com/wpm/Mathdoku/blob/main/LICENSE).
