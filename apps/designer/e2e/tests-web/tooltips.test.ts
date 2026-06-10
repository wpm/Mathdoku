import { test, expect } from '@playwright/test';
import { gotoApp, waitForGrid } from '../tests/helpers';
import { ENTER, SHIFT_ARROW_RIGHT } from '../tests/keys';

// In-app help is tooltips, not documentation (ADR-0007, issue #128). The
// copy lives in src/help.rs and is rendered through native `title`
// attributes (HTML controls) and SVG <title> children (grid elements), so
// the same markup drives the browser tooltip in both the Tauri webview and
// this `--features web` preview build. Like editor_flow.test.ts this spec
// installs NO window.__TAURI__ stubs; it locks in that the tooltips render
// in the web preview exactly as they do on desktop.
test.describe('web build tooltips', () => {
  test('primary controls and operator tabs carry tooltips', async ({
    page,
  }) => {
    await gotoApp(page);

    // New-puzzle modal: the grid-size dropdown and creation button explain
    // themselves via title attributes.
    await expect(
      page.locator('p').filter({ hasText: 'New puzzle' }),
    ).toBeVisible();
    await expect(page.locator('select[title]')).toHaveAttribute(
      'title',
      /grid size/i,
    );
    await expect(
      page.getByRole('button', { name: 'Random Solution', exact: true }),
    ).toHaveAttribute('title', /random solution/i);

    // The grid itself: an SVG <title> child summarizes cage construction.
    await page
      .getByRole('button', { name: 'Random Solution', exact: true })
      .click();
    await waitForGrid(page);
    await expect(page.locator('.grid-svg > title')).toHaveText(/draw a cage/i);

    // Operator tabs: draw the domino {(0,0),(0,1)} and open the selector.
    // Each tab's <g> carries an SVG <title> explaining the operator's
    // constraint.
    await page.locator('.grid-svg').focus();
    await page.keyboard.press(SHIFT_ARROW_RIGHT);
    await page.keyboard.press(ENTER);
    const tabTitles = page.locator('.grid-svg g > title');
    await expect(
      tabTitles.filter({ hasText: /sum to the target/i }),
    ).toHaveCount(1);
    expect(await tabTitles.count()).toBeGreaterThan(1);

    // Commit the Add cage, then the cage-stats readout (solvability
    // indicator for the selected cage) explains its multiset/tuple counts.
    await page
      .locator('.grid-svg text[font-weight="700"]')
      .filter({ hasText: /^\+\d+$/ })
      .first()
      .click();
    await expect(page.locator('.cage-stats')).toHaveAttribute(
      'title',
      /multisets/i,
    );
  });
});
