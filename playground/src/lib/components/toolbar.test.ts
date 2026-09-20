import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect, test } from 'vitest';

const src = readFileSync(join(dirname(fileURLToPath(import.meta.url)), 'Toolbar.svelte'), 'utf8');

test('toolbar is in-flow and keeps e2e hooks', () => {
  expect(src).not.toMatch(/fixed inset-x-0 bottom-0/);
  expect(src).toContain('data-testid="savings"');
  expect(src).toContain('Output format');
  expect(src).toContain('Download');
});
