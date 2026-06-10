import { test, expect, type Page } from '@playwright/test';
import { installTauriStubs, gotoApp, waitForGrid } from './helpers';

// The default build ships With-Solution authoring only: the `without-solution`
// cargo feature is off, so the New-puzzle modal offers no "No Solution" button
// and every created puzzle carries a fixed solution. The mode marker is
// `data-solution-mode` on .puzzle-wrap ("with"/"without").
const mode = (page: Page) => page.locator('.puzzle-wrap[data-solution-mode]');

const modalTitle = (page: Page) =>
  page.locator('p').filter({ hasText: 'New puzzle' });

test.describe('new-puzzle modal (default with-solution-only build)', () => {
  test('modal offers Random Solution and no No Solution button', async ({
    page,
  }) => {
    await installTauriStubs(page, null);
    await gotoApp(page);
    await expect(modalTitle(page)).toBeVisible();

    await expect(
      page.getByRole('button', { name: 'Random Solution', exact: true }),
    ).toBeVisible();
    await expect(
      page.getByRole('button', { name: 'No Solution', exact: true }),
    ).toHaveCount(0);
  });

  test('Random Solution button creates a With-Solution puzzle', async ({
    page,
  }) => {
    await installTauriStubs(page, null);
    await gotoApp(page);
    await expect(modalTitle(page)).toBeVisible();

    await page
      .getByRole('button', { name: 'Random Solution', exact: true })
      .click();
    await waitForGrid(page);

    await expect(mode(page)).toHaveAttribute('data-solution-mode', 'with');
  });
});
