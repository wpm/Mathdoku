import { test, expect, type Page } from '@playwright/test';
import {
  installTauriStubs,
  gotoApp,
  waitForGrid,
  emitTauriEvent,
  PUZZLE_3,
} from './helpers';
import { TAB, SHIFT_TAB } from './keys';

// Keyboard focus for the New-puzzle (size) modal when it opens over an
// existing puzzle (menu New / Cmd-N). The `autofocus` attribute only applies
// during initial document load, so the modal must move focus to the Size
// dropdown programmatically on mount; otherwise focus stays on the grid SVG
// behind the overlay and Tab keeps cycling the cages instead of the modal.

const modalTitle = (page: Page) =>
  page.locator('p').filter({ hasText: 'New puzzle' });

async function openModalOverPuzzle(page: Page) {
  await installTauriStubs(page, PUZZLE_3);
  await gotoApp(page);
  await waitForGrid(page);
  await page.locator('.grid-svg').focus();
  await emitTauriEvent(page, 'menu-new');
  await expect(modalTitle(page)).toBeVisible();
}

test.describe('size modal focus (menu-new over an existing puzzle)', () => {
  test('focus moves to the Size dropdown when the modal opens', async ({
    page,
  }) => {
    await openModalOverPuzzle(page);
    await expect(page.locator('select')).toBeFocused();
    await expect(page.locator('.grid-svg')).not.toBeFocused();
  });

  test('Tab cycles through the modal controls and wraps, never reaching the grid', async ({
    page,
  }) => {
    await openModalOverPuzzle(page);
    // Tab order: select → Create → Cancel → select.
    await page.keyboard.press(TAB);
    await expect(
      page.getByRole('button', { name: 'Create', exact: true }),
    ).toBeFocused();
    await page.keyboard.press(TAB);
    await expect(
      page.getByRole('button', { name: 'Cancel', exact: true }),
    ).toBeFocused();
    await page.keyboard.press(TAB);
    await expect(page.locator('select')).toBeFocused();
  });

  test('Shift+Tab wraps backwards from the dropdown to Cancel', async ({
    page,
  }) => {
    await openModalOverPuzzle(page);
    await page.keyboard.press(SHIFT_TAB);
    await expect(
      page.getByRole('button', { name: 'Cancel', exact: true }),
    ).toBeFocused();
  });

  test('startup (mandatory) modal also focuses the Size dropdown', async ({
    page,
  }) => {
    await installTauriStubs(page, null);
    await gotoApp(page);
    await expect(modalTitle(page)).toBeVisible();
    await expect(page.locator('select')).toBeFocused();
  });
});
