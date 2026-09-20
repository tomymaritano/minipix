import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, test } from 'vitest';

const css = readFileSync(join(dirname(fileURLToPath(import.meta.url)), 'app.css'), 'utf8');

describe('app.css tokens', () => {
  test('uses the spec palette', () => {
    expect(css).toMatch(/#0e1013/i);
    expect(css).toMatch(/#16191e/i);
    expect(css).toMatch(/#1c2026/i);
    expect(css).toMatch(/#ecedef/i);
    expect(css).toMatch(/#8b909a/i);
    expect(css).toMatch(/#3ddc97/i);
    expect(css).toMatch(/#06120d/i);
    expect(css).toMatch(/#e8b44c/i);
    expect(css).toMatch(/#f2685a/i);
  });

  test('toolbar and chrome heights', () => {
    expect(css).toMatch(/--bar-h:\s*56px/);
    expect(css).toMatch(/--chrome-h:\s*48px/);
  });

  test('drops grain and mint glow atmosphere', () => {
    expect(css).not.toMatch(/feTurbulence/);
    expect(css).not.toContain('mint-glow');
  });
});
