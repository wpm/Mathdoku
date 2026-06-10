# ADR-0007: Help system — tooltips in the app, documentation on the website

**Status:** Accepted
**Date:** 2026-06-10
**Deciders:** Bill McNeill (Mathdoku owner)

## Context

Release-plan Phase 9 calls for in-app help in Mathdoku Designer. The original
sketch bundled Markdown files (puzzle rules, basic Designer usage) into the
app, rendered them in a help panel reachable from a Help menu and a `?`
button, and added tooltips on the cage operators and primary controls.

Two facts about the project make that sketch heavier than the need it serves.
First, the Designer's audience is puzzle *designers* — people who already know
the genre and are here to author puzzles, not learn the rules. Second, the
README already promises that end-user gameplay documentation lives on the
project website (ADR-0002's Phase 7 site, now deployed), which means the
conceptual content the bundled panel would carry — what Mathdoku is, how cages
constrain values — has a home that serves players and designers alike, and
maintaining a second bundled copy inside the app creates a synchronization
obligation with no audience to justify it.

A bundled panel also carries real implementation weight in this stack. The
frontend is Leptos CSR compiled to WASM under ADR-0002's 2 MB
brotli-compressed bundle budget. Rendering Markdown in-app means either
shipping a parser (pulldown-cmark) in the bundle or converting help content to
HTML at build time, plus a panel component, topic index, and content that must
track UI changes release after release.

What in-app help must actually accomplish for this audience: answer "what does
this control do" at the moment of confusion, and give the lost user a route to
fuller documentation. Anything beyond that is speculative until real users
demonstrate the need.

## Decision

The app gets tooltips, not documentation. Conceptual and workflow
documentation lives on the website; the app links to it.

Concretely: tooltips on the cage operators and primary controls; a Help menu
(Tauri `MenuBuilder`) whose items open the website's puzzle-rules and
Designer-guide pages in the system browser; and a sample puzzle pre-loaded on
first launch so the initial screen demonstrates the product instead of
explaining it. The website's Designer guide gets stable section anchors
(`…/guide#operators`) so Help menu items — and any future "more…" affordance
on a tooltip — deep-link to the relevant section rather than dumping the user
at a landing page.

No Markdown renderer ships in the app, no help panel component is built, and
no documentation files are bundled. The web preview build hides the Help menu
entries' native-shell aspects the same way it hides Save and Open (ADR-0002);
the tooltips themselves work identically in both builds.

A bundled in-app help panel is recorded as a contingency, not a roadmap item.
The trigger for revisiting: testers or users repeatedly asking workflow
questions ("how do I check uniqueness before exporting?") that have no
hoverable control to carry the answer. Nothing in this decision makes that
retrofit harder; the natural implementation at that point is build-time
Markdown-to-HTML conversion embedded in the binary, indexed by filename
prefix, rendered in a Leptos panel.

## Options Considered

### Option A: Tooltips + Help menu linking to website docs — *chosen*

| Dimension | Assessment |
|-----------|------------|
| Complexity | Low — tooltip attributes, two menu items, anchors on existing website pages |
| Bundle impact | None — no parser, no bundled content |
| Content maintenance | Single copy, on the website, shared with the player audience |
| Offline behavior | Tooltips work; full documentation requires a network connection |

**Pros:** smallest implementation that covers the audience's actual needs;
one documentation source instead of two; web preview gets identical behavior
for free; first-launch sample puzzle does the heaviest onboarding lifting.

**Cons:** offline desktop users have no long-form help; workflow-level
questions have no in-app answer; quality depends on the website's Designer
guide actually being written and anchored.

### Option B: Bundled Markdown help panel

| Dimension | Assessment |
|-----------|------------|
| Complexity | Medium — renderer or build-time conversion, panel component, topic index |
| Bundle impact | Build-time conversion keeps it small; runtime pulldown-cmark adds a parser to the WASM bundle |
| Content maintenance | Second copy of conceptual content, versioned with the app, drifting from the website's |
| Offline behavior | Complete |

**Pros:** help available offline and versioned with the exact UI it
describes; no dependency on website content existing.

**Cons:** real implementation and ongoing content-sync cost serving a
hypothesis about user need; duplicates the website's conceptual content;
panel UI competes with the canvas for attention in a tool whose audience
rarely needs it.

### Option C: Separate help window (Tauri window with webview)

Rejected without a full table: it inherits all of Option B's content costs,
adds window-management code, and does nothing in the web preview, where there
is no second window to open.

## Trade-off Analysis

The decisive consideration is audience. A player-facing game must explain its
rules in-app; a designer-facing tool may assume genre fluency and treat
documentation as reference material, which the platform convention (menu item
opening browser documentation) already handles well. The offline gap is
Option A's only substantive loss, and it lands on the population best
equipped to tolerate it — the in-context tooltips, which are the help that
matters at the moment of use, remain fully offline.

The cost asymmetry seals it: Option A is nearly free and reversible; Option B
is a standing maintenance commitment that, once shipped, cannot be removed
without it reading as a regression. Starting with A and adding B under a
stated trigger gets the cheap version's economics with the thorough version
held in reserve.

## Consequences

- The website's Designer guide and puzzle-rules pages become load-bearing:
  Phase 9 cannot complete before those pages exist with stable anchors
  (tracked in the release plan under Phase 7).
- Help menu items must open the system browser from the Tauri shell —
  a small native affordance with no web-preview equivalent to design.
- Tooltip copy becomes the only in-app documentation surface, so it warrants
  the same review attention as user-facing UI text.
- If the bundled-panel contingency ever triggers, its content should be
  excerpted from the website's guide rather than written fresh, keeping the
  website canonical.
