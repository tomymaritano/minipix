import { test, expect, type Page } from '@playwright/test';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const ROOT = join(here, '..', '..');
const SHOTS = join(here, '_shots');

test.use({ viewport: { width: 1366, height: 820 } });

async function settle(page: Page): Promise<void> {
  await expect(page.locator('img[alt="Compressed"]')).toBeVisible({ timeout: 120_000 });
  await page.waitForTimeout(400);
}

test('shot: empty state', async ({ page }) => {
  await page.goto('/');
  await page.waitForTimeout(900); // dejar correr el reveal
  await page.screenshot({ path: join(SHOTS, 'empty.png') });
});

test('shot: png result', async ({ page }) => {
  await page.goto('/');
  await page
    .locator('input[type="file"]')
    .setInputFiles(join(ROOT, 'tests/vectors/gradient_circle.png'));
  await settle(page);
  await page.screenshot({ path: join(SHOTS, 'result-png.png') });
});

test('shot: avif result with savings', async ({ page }) => {
  await page.goto('/');
  await page.getByLabel('Output format').click();
  await page.getByRole('option', { name: 'AVIF' }).click();
  await page
    .locator('input[type="file"]')
    .setInputFiles(join(ROOT, 'tests/vectors/flat_colors.png'));
  await settle(page);
  await page.screenshot({ path: join(SHOTS, 'result-avif.png') });
});
