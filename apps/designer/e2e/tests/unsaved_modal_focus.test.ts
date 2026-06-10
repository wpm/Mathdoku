import { test, expect, type Page } from '@playwright/test';
import {
  installTauriStubs,
  gotoApp,
  waitForGrid,
  emitTauriEvent,
  PUZZLE_3,
} from './helpers';
import { TAB, SHIFT_TAB, ESCAPE } from './keys';

// Keyboard focus for the unsaved-changes modal (Cmd-W / window close request).
// The modal must move focus to the Save button on mount and trap Tab inside
// its three buttons; otherwise focus stays on (or escapes back to) the grid
// SVG behind the overlay and Tab keeps cycling the cages instead of the modal.

const modalTitle = (page: Page) =>
  page.locator('p').filter({ hasText: 'Save changes before closing?' });

async function openModalOverPuzzle(page: Page) {
  await installTauriStubs(page, PUZZLE_3);
  await gotoApp(page);
  await waitForGrid(page);
  await page.locator('.grid-svg').focus();
  await emitTauriEvent(page, 'request-close');
  await expect(modalTitle(page)).toBeVisible();
}

test.describe('unsaved-changes modal focus (close request over a puzzle)', () => {
  test('focus moves to the Save button when the modal opens', async ({
    page,
  }) => {
    await openModalOverPuzzle(page);
    await expect(
      page.getByRole('button', { name: 'Save', exact: true }),
    ).toBeFocused();
    await expect(page.locator('.grid-svg')).not.toBeFocused();
  });

  test('Tab cycles through the modal buttons and wraps, never reaching the grid', async ({
    page,
  }) => {
    await openModalOverPuzzle(page);
    // DOM order: Don't Save → Cancel → Save. Focus starts on Save, so the
    // first Tab wraps around to Don't Save.
    await page.keyboard.press(TAB);
    await expect(
      page.getByRole('button', { name: 'Don’t Save', exact: true }),
    ).toBeFocused();
    await page.keyboard.press(TAB);
    await expect(
      page.getByRole('button', { name: 'Cancel', exact: true }),
    ).toBeFocused();
    await page.keyboard.press(TAB);
    await expect(
      page.getByRole('button', { name: 'Save', exact: true }),
    ).toBeFocused();
    await expect(page.locator('.grid-svg')).not.toBeFocused();
  });

  test('Shift+Tab wraps backwards from Save to Cancel', async ({ page }) => {
    await openModalOverPuzzle(page);
    await page.keyboard.press(SHIFT_TAB);
    await expect(
      page.getByRole('button', { name: 'Cancel', exact: true }),
    ).toBeFocused();
  });

  test('Escape cancels the close request and dismisses the modal', async ({
    page,
  }) => {
    await openModalOverPuzzle(page);
    await page.keyboard.press(ESCAPE);
    await expect(modalTitle(page)).not.toBeVisible();
    await expect(page.locator('.grid-svg')).toBeVisible();
  });
});
