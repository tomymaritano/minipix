// Smoke de conformance del build wasm. Correr: node crates/wasm/tests/smoke.mjs
// (antes: crates/wasm/build-node-test.ps1)
//
// Decisiones documentadas:
// (a) AVIF: wasm usa rav1e con asm deshabilitado (wasm32); nativo usa asm. Los hashes
//     DIFIEREN entre wasm y nativo. Se valida que el output sea structuralmente correcto
//     (AVIF válido, tamaño razonable) en lugar de byte-igual al golden nativo.
// (b) WebP lossless: nativo usa libwebp-C; wasm usa image-webp VP8L (Rust puro).
//     Los bytes SON DIFERENTES. Se usa goldens-wasm.json para los hashes wasm-específicos.
//     Este archivo es generado por este mismo script con MINIPIX_REGEN_WASM_GOLDENS=1.
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { createHash } from 'node:crypto';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { createRequire } from 'node:module';

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..', '..');
const require = createRequire(import.meta.url);
const wasmMod = require(join(root, 'target', 'wasm-node-test', 'minipix_wasm.js'));
const { compress, convert, encodeRgba } = wasmMod;

const goldens = JSON.parse(readFileSync(join(root, 'tests/conformance/goldens.json'), 'utf8'));
const sha = (buf) => createHash('sha256').update(Buffer.from(buf)).digest('hex');

// --- Gestión de goldens wasm-específicos ---
const wasmGoldensPath = join(root, 'tests/conformance/goldens-wasm.json');
let wasmGoldens = {};
try {
  wasmGoldens = JSON.parse(readFileSync(wasmGoldensPath, 'utf8'));
} catch {
  // goldens-wasm.json no existe aún; se creará en modo REGEN
}
const REGEN = process.env.MINIPIX_REGEN_WASM_GOLDENS === '1';
const wasmGoldensNew = {};

// --- Resultados AVIF para decisión (a) ---
const avifResults = [];

for (const vector of ['gradient_circle', 'flat_colors']) {
  const input = readFileSync(join(root, `tests/vectors/${vector}.png`));

  // PNG: camino 100% puro-Rust idéntico al nativo → byte-igual a goldens nativos.
  const c = compress(input, {});
  const pngHash = sha(c.data);
  assert.equal(
    pngHash,
    goldens[`${vector}.compress.png.q75e4`],
    `${vector} png parity wasm==native`
  );
  console.log(`[OK] ${vector} png: wasm == native golden`);

  // AVIF: paridad byte a byte wasm==native VERIFICADA (rav1e asm-off == asm-on,
  // 2026-06-12). Assert duro contra el golden nativo: si una versión futura de
  // rav1e rompe la paridad, este test DEBE fallar para forzar una decisión
  // explícita (golden wasm separado o investigación).
  const avif = convert(input, { format: 'avif', effort: 4 });
  const avifHash = sha(avif.data);
  assert.equal(
    avifHash,
    goldens[`${vector}.convert.avif.q75e4`],
    `${vector} AVIF parity wasm==native rota — decidir explícitamente (ver comentario)`
  );
  avifResults.push({ vector, match: true, wasmHash: avifHash.slice(0, 16), nativeHash: goldens[`${vector}.convert.avif.q75e4`].slice(0, 16) });
  console.log(`[OK] ${vector} avif: wasm == native golden`);

  // JPEG: encoder distinto por diseño (jpeg-encoder vs mozjpeg) — sanity only.
  const jpg = convert(input, { format: 'jpeg' });
  assert.ok(jpg.bytesOut > 0 && jpg.format === 'jpeg', `${vector} jpeg sanity`);
  console.log(`[OK] ${vector} jpeg: sanity ok (${jpg.bytesOut} bytes)`);

  // WebP lossless: nativo=libwebp-C (RIFF VP8L), wasm=image-webp (VP8L Rust).
  // Bytes DIFERENTES confirmado en Step 1 (goldens nativos ya en goldens.json).
  // Verificación: contra goldens-wasm.json (generado desde el build wasm).
  const webp = convert(input, { format: 'webp', lossless: true });
  const webpHash = sha(webp.data);
  wasmGoldensNew[`${vector}.convert.webp.lossless`] = webpHash;

  if (REGEN) {
    console.log(`[REGEN] ${vector} webp lossless wasm hash: ${webpHash.slice(0, 16)}`);
  } else if (wasmGoldens[`${vector}.convert.webp.lossless`]) {
    assert.equal(
      webpHash,
      wasmGoldens[`${vector}.convert.webp.lossless`],
      `${vector} webp lossless wasm golden`
    );
    // Comparar con nativo para confirmar que difieren (documentación)
    const nativeLosslessGolden = goldens[`${vector}.convert.webp.lossless`];
    if (nativeLosslessGolden) {
      const wasmMatchesNative = webpHash === nativeLosslessGolden;
      console.log(`[INFO] ${vector} webp lossless: wasm==${webpHash.slice(0, 16)} native==${nativeLosslessGolden.slice(0, 16)} same_bytes=${wasmMatchesNative}`);
    }
    console.log(`[OK] ${vector} webp lossless: wasm golden match`);
  } else {
    // goldens-wasm.json existe pero falta esta clave → REGEN pendiente
    assert.ok(webp.bytesOut > 0, `${vector} webp lossless wasm sanity`);
    console.log(`[WARN] ${vector} webp lossless: golden-wasm no encontrado, sanity only`);
  }

  // encodeRgba:
  const raw = new Uint8Array(4 * 4 * 4).fill(128);
  const er = encodeRgba(raw, 4, 4, 'png', {});
  assert.ok(er.bytesOut > 0, 'encodeRgba png');
  console.log(`[OK] encodeRgba png: ${er.bytesOut} bytes`);
}

// --- Regenerar goldens-wasm.json si se pidió ---
if (REGEN) {
  mkdirSync(join(root, 'tests/conformance'), { recursive: true });
  writeFileSync(wasmGoldensPath, JSON.stringify(wasmGoldensNew, null, 2) + '\n', 'utf8');
  console.log(`[REGEN] goldens-wasm.json escrito en: ${wasmGoldensPath}`);
}

// --- Errores tipados ---
assert.throws(() => compress(new Uint8Array([1, 2, 3]), {}), /UnsupportedFormat/);
console.log('[OK] error UnsupportedFormat tipado');

assert.throws(
  () => convert(readFileSync(join(root, 'tests/vectors/flat_colors.png')), { format: 'webp', quality: 80 }),
  /InvalidOptions/,
  'lossy webp debe rechazarse en wasm'
);
console.log('[OK] error InvalidOptions (lossy webp en wasm) tipado');

// AVIF decode rechazado en wasm (el navegador decodifica):
// generar un avif chico con convert y intentar comprimirlo (compress = decode+reencode del mismo formato):
const tinyAvif = convert(readFileSync(join(root, 'tests/vectors/flat_colors.png')), { format: 'avif', effort: 4 });
assert.throws(
  () => compress(tinyAvif.data, {}),
  /DecodeError|UnsupportedFormat|not available/i,
  'avif decode rechazado en wasm'
);
console.log('[OK] avif decode rechazado en wasm (esperado)');

// --- Resumen decisiones ---
console.log('');
console.log('=== Decisiones de conformance ===');
const avifAllMatch = avifResults.every(r => r.match);
if (avifAllMatch) {
  console.log('(a) AVIF: PARIDAD TOTAL wasm==native. Los hashes coinciden. (rav1e sin asm produce output idéntico al nativo en este runner)');
} else {
  console.log('(a) AVIF: wasm DIFIERE de nativo (esperado: rav1e asm-on vs asm-off). Se validó tamaño/formato. Ver detalle arriba.');
}
console.log('(b) WebP lossless: wasm usa image-webp VP8L (Rust); nativo usa libwebp-C. Bytes distintos. goldens-wasm.json contiene los hashes wasm-específicos.');
console.log('');
console.log('wasm conformance: OK');
