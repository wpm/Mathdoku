# Changelog

All notable changes to the `mathdoku` crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!--
Record changes here as they land, grouped under the relevant heading:
Added, Changed, Deprecated, Removed, Fixed, Security. When a version is
released, move the accumulated entries from [Unreleased] into a new dated
section and update the comparison links at the bottom of the file.
-->

## [Unreleased]

## [0.2.0](https://github.com/wpm/Mathdoku/compare/mathdoku-v0.1.0...mathdoku-v0.2.0) - 2026-06-16

### Other

- extract shared row-sum 3x3 fixture and tidy designer tests ([#161](https://github.com/wpm/Mathdoku/pull/161))
- simplify puzzle test setup with helper functions ([#155](https://github.com/wpm/Mathdoku/pull/155))
- make operators_for a public method of Polyomino ([#150](https://github.com/wpm/Mathdoku/pull/150))
- remove Operator alias, use CageOperator everywhere ([#149](https://github.com/wpm/Mathdoku/pull/149))
- make generate private to the mathdoku crate ([#148](https://github.com/wpm/Mathdoku/pull/148))
- [**breaking**] rename T to Target and type MDD depth as usize ([#146](https://github.com/wpm/Mathdoku/pull/146))
- complete mathdoku crate publish metadata ([#145](https://github.com/wpm/Mathdoku/pull/145))
- remove dead-code suppressors across mathdoku and designer ([#140](https://github.com/wpm/Mathdoku/pull/140))
- extract test fixtures into shared modules ([#139](https://github.com/wpm/Mathdoku/pull/139))
- Fix CageOperator documentation ([#136](https://github.com/wpm/Mathdoku/pull/136))
- ignore fat-cage perf regression by default ([#135](https://github.com/wpm/Mathdoku/pull/135))
- rename neighbors_4 to edge_adjacent_cells ([#134](https://github.com/wpm/Mathdoku/pull/134))
- add crate README and split Codecov coverage per flag ([#120](https://github.com/wpm/Mathdoku/pull/120))

## [0.1.0] - Unreleased

_Placeholder for the first published release. Entries from [Unreleased] will be
moved here, and a release date added, when `mathdoku` 0.1.0 is tagged and
published to crates.io._

[Unreleased]: https://github.com/wpm/Mathdoku/compare/mathdoku-v0.1.0...HEAD
[0.1.0]: https://github.com/wpm/Mathdoku/releases/tag/mathdoku-v0.1.0
