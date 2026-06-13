import { test, expect, type Page } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const ROOT = join(here, '..', '..');
const sha = (b: Buffer | Uint8Array): string => createHash('sha256').update(b).digest('hex');
const goldens = JSON.parse(
  readFileSync(join(ROOT, 'tests/conformance/goldens.json'), 'utf8'),
) as Record<string, string>;

/**
 * El comparador renderiza el resultado como <img alt="Compressed"> con un blob URL.
 * Esperamos a que aparezca y leemos sus bytes vía fetch en el contexto de la página
 * (los blob: URLs no disparan el evento download de Playwright de forma fiable).
 */
async function compressedBytes(page: Page): Promise<Buffer> {
  const img = page.locator('img[alt="Compressed"]');
  await expect(img).toBeVisible({ timeout: 120_000 });
  const src = await img.getAttribute('src');
  if (!src) throw new Error('compressed image has no src');
  const bytes = await page.evaluate(async (url: string) => {
    const r = await fetch(url);
    const b = await r.arrayBuffer();
    return Array.from(new Uint8Array(b));
  }, src);
  return Buffer.from(bytes);
}

/**
 * shadcn Select renders as a button+listbox (not a native <select>).
 * Click the trigger (aria-label="Output format"), then click the option.
 */
async function selectFormat(page: Page, fmt: string): Promise<void> {
  await page.getByLabel('Output format').click();
  await page.getByRole('option', { name: fmt }).click();
}

test('comprime un PNG con paridad de goldens (camino puro-Rust en el browser)', async ({
  page,
}) => {
  await page.goto('/');
  // PNG dropeado → el selector toma PNG, quality 75 / effort 4 (defaults) = el golden compress.
  await page
    .locator('input[type="file"]')
    .setInputFiles(join(ROOT, 'tests/vectors/gradient_circle.png'));

  const bytes = await compressedBytes(page);
  expect(sha(bytes)).toBe(goldens['gradient_circle.compress.png.q75e4']);

  // La UI muestra el ahorro y un botón de descarga.
  await expect(page.locator('[data-testid="savings"]')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Download' })).toBeVisible();
});

test('convierte PNG a AVIF en el browser (paridad de goldens)', async ({ page }) => {
  await page.goto('/');
  await selectFormat(page, 'AVIF'); // elegir formato antes del drop
  await page
    .locator('input[type="file"]')
    .setInputFiles(join(ROOT, 'tests/vectors/flat_colors.png'));

  const bytes = await compressedBytes(page);
  // Paridad AVIF wasm==native verificada (T4) — assert duro.
  expect(sha(bytes)).toBe(goldens['flat_colors.convert.avif.q75e4']);
});

test('acepta AVIF de entrada y convierte a JPEG (decode por el navegador)', async ({ page }) => {
  await page.goto('/');
  await selectFormat(page, 'JPEG');
  await page.locator('input[type="file"]').setInputFiles(join(here, 'fixtures/tiny.avif'));

  const bytes = await compressedBytes(page);
  // Magic bytes JPEG: FF D8 FF.
  expect(bytes[0]).toBe(0xff);
  expect(bytes[1]).toBe(0xd8);
  expect(bytes[2]).toBe(0xff);
});

test('el botón Download entrega un archivo con la extensión correcta', async ({ page }) => {
  await page.goto('/');
  await page
    .locator('input[type="file"]')
    .setInputFiles(join(ROOT, 'tests/vectors/gradient_circle.png'));
  await expect(page.locator('img[alt="Compressed"]')).toBeVisible({ timeout: 120_000 });

  const downloadPromise = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Download' }).click();
  const download = await downloadPromise;
  expect(download.suggestedFilename()).toMatch(/gradient_circle\.png$/);
});
