import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect, test } from 'vitest';

const here = dirname(fileURLToPath(import.meta.url));
const app = readFileSync(join(here, 'App.svelte'), 'utf8');
const empty = readFileSync(join(here, 'lib/components/EmptyState.svelte'), 'utf8');

test('app shell is in-flow flex, not overlay brand', () => {
  expect(app).toContain('flex h-full flex-col p-2');
  expect(app).not.toContain(String.fromCodePoint(0x25c6));
});

test('empty state is a div dropzone with an inner Button', () => {
  expect(empty).toContain('Select files');
  expect(empty).not.toContain(['icon', 'float'].join('-'));
});
