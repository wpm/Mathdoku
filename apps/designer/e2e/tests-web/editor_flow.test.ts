import { test, expect } from '@playwright/test';
import { gotoApp, waitForGrid } from '../tests/helpers';
import { ENTER, SHIFT_ARROW_RIGHT } from '../tests/keys';

// End-to-end smoke test of the WASM-everything preview build (issue #74).
//
// Unlike every spec under ../tests, this one installs NO window.__TAURI__
// stubs: each puzzle-state command runs in-process against
// mathdoku-designer-core through the `#[cfg(feature = "web")]` ipc bodies and
// the thread-local AppState store. It locks in that the migration actually made
// the preview editor functional — create → select → insert cage — rather than
// just rendering chrome. Driven by playwright.web.config.ts, which serves a
// `trunk serve --features web` build on :1421.
test.describe('web build editor flow', () => {
  test('create puzzle, select cells, insert cage', async ({ page }) => {
    await gotoApp(page);

    // With no puzzle in the thread-local store, get_puzzle returns None and
    // the New-puzzle modal appears. Create a With-Solution puzzle
    // (new_latin_square) — the default build has no Without-Solution path.
    await expect(
      page.locator('p').filter({ hasText: 'New puzzle' }),
    ).toBeVisible();
    await page.getByRole('button', { name: 'Create', exact: true }).click();
    await waitForGrid(page);
    await expect(page.locator('.puzzle-wrap')).toHaveAttribute(
      'data-solution-mode',
      'with',
    );

    // Selection (set_active_cell): focus the grid, draw the domino {(0,0),(0,1)}.
    await page.locator('.grid-svg').focus();
    await page.keyboard.press(SHIFT_ARROW_RIGHT);
    await page.keyboard.press(ENTER);

    // Insert cage (insert_cage): the operator tabs carry solution-derived
    // targets ("+N", "−N", …); the exact numbers depend on the random
    // solution, so pick the Add tab by its "+" prefix.
    const boldLabels = page.locator('.grid-svg text[font-weight="700"]');
    await boldLabels
      .filter({ hasText: /^\+\d+$/ })
      .first()
      .click();

    // The committed cage's op label proves insert_cage round-tripped through
    // core: the selector tabs are gone and only the single "+N" label remains.
    await expect(boldLabels).toHaveCount(1);
    await expect(boldLabels.filter({ hasText: /^\+\d+$/ })).toHaveCount(1);
  });
});
