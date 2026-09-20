import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, test } from 'vitest';

const dir = dirname(fileURLToPath(import.meta.url));
const read = (rel: string): string => readFileSync(join(dir, rel), 'utf8');

describe('ui primitives', () => {
  test('do not use mint-glow', () => {
    for (const rel of [
      'button/button.svelte',
      'badge/badge.svelte',
      'select/select.svelte',
      'slider/slider.svelte',
      'switch/switch.svelte',
    ]) {
      expect(read(rel), rel).not.toContain('mint-glow');
    }
  });

  test('signal color goes through the primary token class', () => {
    expect(read('button/button.svelte')).toContain('bg-primary');
    expect(read('badge/badge.svelte')).toContain('text-primary');
    expect(read('slider/slider.svelte')).toContain('bg-primary');
    expect(read('switch/switch.svelte')).toContain('bg-primary');
  });

  test('select keeps the e2e aria-label forwarding', () => {
    expect(read('select/select.svelte')).toContain("'aria-label'");
    expect(read('select/select.svelte')).toContain('aria-label={ariaLabel}');
  });
});
