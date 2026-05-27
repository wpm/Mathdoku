import { test, expect, type Page } from '@playwright/test';
import { installTauriStubs, gotoApp, waitForGrid } from './helpers';
import { ENTER, SHIFT_ARROW_RIGHT } from './keys';

const fixButton = (page: Page) => page.getByRole('button', { name: 'Fix', exact: true });
const unfixButton = (page: Page) => page.getByRole('button', { name: 'Unfix', exact: true });
const selectorLabels = (page: Page) => page.locator('.grid-svg text[font-weight="700"]');

test.describe('new-puzzle modal authoring mode', () => {
  test('Empty button creates a Without-Solution puzzle', async ({ page }) => {
    await installTauriStubs(page, null);
    await gotoApp(page);
    await expect(page.locator('p').filter({ hasText: 'New puzzle' })).toBeVisible();

    await page.getByRole('button', { name: 'Empty', exact: true }).click();
    await waitForGrid(page);

    // Without-Solution mode offers the Fix control, not Unfix.
    await expect(fixButton(page)).toBeVisible();
    await expect(unfixButton(page)).toHaveCount(0);
  });

  test('With Solution button creates a With-Solution puzzle', async ({ page }) => {
    await installTauriStubs(page, null);
    await gotoApp(page);
    await expect(page.locator('p').filter({ hasText: 'New puzzle' })).toBeVisible();

    await page.getByRole('button', { name: 'With Solution', exact: true }).click();
    await waitForGrid(page);

    await expect(unfixButton(page)).toBeVisible();
    await expect(fixButton(page)).toHaveCount(0);
  });
});

test.describe('fix / unfix mode switching', () => {
  test('Unfix drops the solution and Fix snapshots it back', async ({ page }) => {
    // Default stub puzzle is With-Solution.
    await installTauriStubs(page, { n: 5 });
    await gotoApp(page);
    await waitForGrid(page);

    await expect(unfixButton(page)).toBeVisible();
    await unfixButton(page).click();
    await expect(fixButton(page)).toBeVisible();

    await fixButton(page).click();
    await expect(unfixButton(page)).toBeVisible();
  });
});

test.describe('Without-Solution cage commit', () => {
  test('two-step picker commits a cage with an author-chosen target', async ({ page }) => {
    await installTauriStubs(page, { n: 3 }, { withoutSolution: true });
    await gotoApp(page);
    await waitForGrid(page);
    await page.locator('.grid-svg').focus();

    // Draw the pair {(0,0),(0,1)} and open the operation selector.
    await page.keyboard.press(SHIFT_ARROW_RIGHT);
    await page.keyboard.press(ENTER);

    // Step one: the operator strip. An empty 3×3 pair admits an Add target.
    await expect(selectorLabels(page).filter({ hasText: /^\+$/ })).toBeVisible();
    await selectorLabels(page).filter({ hasText: /^\+$/ }).click();

    // Step two: the target sub-picker. {1,2} sums to 3, so +3 is feasible.
    await expect(selectorLabels(page).filter({ hasText: /^\+3$/ })).toBeVisible();
    await selectorLabels(page).filter({ hasText: /^\+3$/ }).click();

    // The committed cage shows its +3 label and the provisional outline is gone.
    await expect(page.locator('.grid-svg text').filter({ hasText: /^\+3$/ })).toBeVisible();
  });
});

test.describe('Without-Solution singleton cages', () => {
  test('typing a permitted digit immediately creates a singleton cage', async ({ page }) => {
    await installTauriStubs(page, { n: 3 }, { withoutSolution: true });
    await gotoApp(page);
    await waitForGrid(page);
    await page.locator('.grid-svg').focus();

    // The active cell starts at (0,0); 2 is a permitted value in an empty 3×3.
    await page.keyboard.press('2');

    // A committed Given cage labelled "2" appears with no selector step. Cage and
    // selector labels are weight 700, isolating it from the cell's domain digits.
    await expect(selectorLabels(page).filter({ hasText: /^2$/ })).toBeVisible();
  });

  test('singleton picker opens on the value dropdown, skipping the operator step', async ({ page }) => {
    await installTauriStubs(page, { n: 3 }, { withoutSolution: true });
    await gotoApp(page);
    await waitForGrid(page);
    await page.locator('.grid-svg').focus();

    // Enter on the empty active cell opens the singleton picker directly.
    await page.keyboard.press(ENTER);

    // The dropdown lists numeric values (1, 2, 3) — no '#' operator header.
    await expect(selectorLabels(page).filter({ hasText: /^1$/ })).toBeVisible();
    await expect(selectorLabels(page).filter({ hasText: /^3$/ })).toBeVisible();
    await expect(selectorLabels(page).filter({ hasText: '#' })).toHaveCount(0);

    // Picking a value commits the singleton cage with that target.
    await selectorLabels(page).filter({ hasText: /^3$/ }).click();
    await expect(selectorLabels(page).filter({ hasText: /^3$/ })).toBeVisible();
  });
});
