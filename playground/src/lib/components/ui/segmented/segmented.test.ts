import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, test } from 'vitest';

const src = readFileSync(join(dirname(fileURLToPath(import.meta.url)), 'segmented.svelte'), 'utf8');

describe('Segmented', () => {
  test('exposes pressed state and value callback', () => {
    expect(src).toContain('aria-pressed');
    expect(src).toContain('onValueChange');
    expect(src).toContain('aria-label');
  });
});
