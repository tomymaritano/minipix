import { compress, convert, compressSync, convertSync } from '../index.js';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..', '..');
const goldens = JSON.parse(readFileSync(join(root, 'tests/conformance/goldens.json'), 'utf8'));
const sha = (buf) => createHash('sha256').update(buf).digest('hex');

// ── conformance async ──────────────────────────────────────────────────────
for (const vector of ['gradient_circle', 'flat_colors']) {
  const input = readFileSync(join(root, `tests/vectors/${vector}.png`));
  const c = await compress(input, {});
  assert.equal(sha(c.data), goldens[`${vector}.compress.png.q75e4`], `${vector} compress png`);
  assert.ok(c.width > 0 && c.bytesOut > 0 && c.ratio > 0);
  for (const fmt of ['jpeg', 'webp', 'avif']) {
    const out = await convert(input, { format: fmt, effort: 4 });
    assert.equal(sha(out.data), goldens[`${vector}.convert.${fmt}.q75e4`], `${vector} -> ${fmt}`);
  }
}

// ── conformance sync ───────────────────────────────────────────────────────
// Fix 3: compressSync/convertSync deben producir bytes idénticos al core Rust.
for (const vector of ['gradient_circle', 'flat_colors']) {
  const input = readFileSync(join(root, `tests/vectors/${vector}.png`));
  const s = compressSync(input, {});
  assert.equal(sha(s.data), goldens[`${vector}.compress.png.q75e4`], `${vector} compressSync png`);
  assert.ok(s.width > 0 && s.bytesOut > 0 && s.ratio > 0, `${vector} compressSync metadata`);
}

// ── errores tipados (async) ────────────────────────────────────────────────
// napi puede lanzar síncronamente antes de crear el AsyncTask,
// por lo que se envuelve en una función async para que assert.rejects reciba una Promise.
await assert.rejects(async () => compress(Buffer.from('garbage'), {}), /unsupported/i);
await assert.rejects(async () => convert(Buffer.from('garbage'), { format: 'bmp' }), /unknown format/i);

// ── código de error estable — Fix 2 ───────────────────────────────────────
// Task::compute está tipado como Result<T, Error<Status>>; el tipo S está fijado
// por el trait, así que err.code siempre será "GenericFailure". En cambio el
// mensaje lleva el prefijo [Código] como contrato público estable.
try {
  await compress(Buffer.from('garbage'), {});
  assert.fail('debio rechazar');
} catch (err) {
  assert.ok(
    err.code === 'UnsupportedFormat' || err.message.startsWith('[UnsupportedFormat]'),
    `codigo estable esperado, fue code=${err.code} msg=${err.message}`,
  );
}

// ── errores tipados (sync) ─────────────────────────────────────────────────
assert.throws(() => compressSync(Buffer.from('garbage'), {}), /unsupported/i);
assert.throws(() => convertSync(Buffer.from('garbage'), { format: 'bmp' }), /unknown format/i);

console.log('node conformance: OK (bytes identicos al core Rust)');
