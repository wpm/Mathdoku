import { test, expect, type Page } from '@playwright/test';
import { installTauriStubs, gotoApp, waitForGrid } from './helpers';
import { ENTER, SHIFT_ARROW_RIGHT } from './keys';

// Browsers render native tooltips only for <title> elements in the SVG
// namespace. The Leptos view! macro treats `title` as ambiguous between HTML
// and SVG and has been observed to emit HTML-namespaced elements inside SVG
// trees, which display nothing (issue #130). These tests pin the namespace so
// a Leptos upgrade or a refactor back to view!-macro syntax can't silently
// kill the tooltips again.

const SVG_NS = 'http://www.w3.org/2000/svg';

const titleNamespaces = (page: Page) =>
  page.$$eval('svg title', (els) => els.map((el) => el.namespaceURI));

async function setup(page: Page) {
  await installTauriStubs(page, { n: 3 });
  await gotoApp(page);
  await waitForGrid(page);
  await page.locator('.grid-svg').focus();
}

test.describe('tooltip namespaces (#130)', () => {
  test('grid tooltip <title> is SVG-namespaced', async ({ page }) => {
    await setup(page);

    const namespaces = await titleNamespaces(page);
    expect(namespaces.length).toBeGreaterThan(0);
    expect([...new Set(namespaces)]).toEqual([SVG_NS]);
  });

  test('operator tab <title>s exist and are SVG-namespaced', async ({
    page,
  }) => {
    await setup(page);
    // Draw a two-cell provisional cage and open the operation selector.
    await page.keyboard.press(SHIFT_ARROW_RIGHT);
    await page.keyboard.press(ENTER);

    const namespaces = await titleNamespaces(page);
    // Grid tooltip plus one per operator tab.
    expect(namespaces.length).toBeGreaterThan(1);
    expect([...new Set(namespaces)]).toEqual([SVG_NS]);

    // The tab tooltips carry the operator explanations.
    const texts = await page.$$eval('svg title', (els) =>
      els.map((el) => el.textContent ?? ''),
    );
    expect(texts.some((t) => t.startsWith('Add:'))).toBe(true);
  });
});
