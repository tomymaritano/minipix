import { test, expect } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const ROOT = join(__dirname, '..', '..');
const sha = (b: Buffer | Uint8Array) => createHash('sha256').update(b).digest('hex');
const goldens = JSON.parse(
  readFileSync(join(ROOT, 'tests/conformance/goldens.json'), 'utf8'),
) as Record<string, string>;

// The download anchor uses a blob URL with the `download` attribute.
// In Playwright, clicking a download link triggers a 'download' event only when
// the page navigates away (blob: URLs don't trigger it by default in Chromium).
// We intercept the blob bytes directly via page.evaluate instead.
async function captureDownloadBytes(
  page: import('@playwright/test').Page,
  downloadLocator: import('@playwright/test').Locator,
): Promise<Buffer> {
  // Grab the href (blob: URL) before clicking
  const href = await downloadLocator.getAttribute('href');
  if (!href) throw new Error('Download link has no href');

  // Fetch the blob content via page context
  const bytes = await page.evaluate(async (url: string) => {
    const response = await fetch(url);
    const buf = await response.arrayBuffer();
    return Array.from(new Uint8Array(buf));
  }, href);

  return Buffer.from(bytes);
}

test('comprime un PNG con paridad de goldens (camino puro-Rust hasta el browser)', async ({
  page,
}) => {
  await page.goto('/');

  // Use auto format (default) = compress, q75 e4
  const input = page.locator('input[type="file"]');
  await input.setInputFiles(join(ROOT, 'tests/vectors/gradient_circle.png'));

  // Wait for the Done badge to appear
  await expect(page.locator('.badge--done').first()).toBeVisible({ timeout: 60_000 });

  // Capture download bytes directly from blob URL
  const downloadLink = page.locator('a.btn-download').first();
  await expect(downloadLink).toBeVisible();

  const bytes = await captureDownloadBytes(page, downloadLink);
  expect(sha(bytes)).toBe(goldens['gradient_circle.compress.png.q75e4']);
});

test('convierte PNG a AVIF en el browser', async ({ page }) => {
  await page.goto('/');

  // Select AVIF format BEFORE uploading
  await page.locator('#format-select').selectOption('avif');

  const input = page.locator('input[type="file"]');
  await input.setInputFiles(join(ROOT, 'tests/vectors/flat_colors.png'));

  // AVIF encode is slow — wait up to 120s
  await expect(page.locator('.badge--done').first()).toBeVisible({ timeout: 120_000 });

  const downloadLink = page.locator('a.btn-download').first();
  await expect(downloadLink).toBeVisible();

  const bytes = await captureDownloadBytes(page, downloadLink);
  // AVIF parity wasm==native verified (smoke.mjs / T4) — hard assert:
  expect(sha(bytes)).toBe(goldens['flat_colors.convert.avif.q75e4']);
});

test('acepta AVIF input y convierte a JPEG (sanity magic bytes)', async ({ page }) => {
  await page.goto('/');

  // Select JPEG output format
  await page.locator('#format-select').selectOption('jpeg');

  const input = page.locator('input[type="file"]');
  await input.setInputFiles(join(__dirname, 'fixtures/tiny.avif'));

  // Wait for done
  await expect(page.locator('.badge--done').first()).toBeVisible({ timeout: 60_000 });

  const downloadLink = page.locator('a.btn-download').first();
  await expect(downloadLink).toBeVisible();

  const bytes = await captureDownloadBytes(page, downloadLink);
  // JPEG magic bytes: FFD8FF
  expect(bytes[0]).toBe(0xff);
  expect(bytes[1]).toBe(0xd8);
  expect(bytes[2]).toBe(0xff);
});
