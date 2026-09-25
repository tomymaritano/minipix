#!/usr/bin/env node
// Compare minipix, sharp, and Pillow on one image.
//
// Quality is set to 75 on each library's own scale. Those scales are not
// visually equivalent. AVIF in minipix is rav1e, one thread, so the output
// stays byte-identical across machines; sharp's AVIF encoder is not rav1e.
//
// Usage (from the repo root): node scripts/compare-bench.mjs

import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync, existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const benchDir = join(root, 'scripts', 'compare-bench');
const cacheDir = join(benchDir, 'cache');
const photoUrl =
  'https://upload.wikimedia.org/wikipedia/commons/thumb/d/d9/Collage_of_Nine_Dogs.jpg/1280px-Collage_of_Nine_Dogs.jpg';
const photoPath = join(cacheDir, '1280px-Collage_of_Nine_Dogs.jpg');
const outDir = join(cacheDir, 'out');

const RUNS = 3;

function fail(message) {
  console.error(message);
  process.exit(1);
}

function run(cmd, args, opts = {}) {
  const result = spawnSync(cmd, args, { encoding: 'utf8', ...opts });
  if (result.status !== 0) {
    fail(
      `${cmd} ${args.join(' ')} failed (${result.status})\n${result.stderr || ''}${result.stdout || ''}`,
    );
  }
  return result.stdout;
}

function median(values) {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length / 2)];
}

async function ensurePhoto() {
  mkdirSync(cacheDir, { recursive: true });
  if (existsSync(photoPath)) return;
  const response = await fetch(photoUrl, {
    headers: { 'user-agent': 'minipix-compare-bench/0.1' },
  });
  if (!response.ok) fail(`download ${photoUrl} -> ${response.status}`);
  writeFileSync(photoPath, Buffer.from(await response.arrayBuffer()));
}

function ensureSharp() {
  const sharpPkg = join(benchDir, 'node_modules', 'sharp', 'package.json');
  if (!existsSync(sharpPkg)) {
    run('npm', ['install', '--ignore-scripts=false'], { cwd: benchDir });
  }
  const require = createRequire(join(benchDir, 'package.json'));
  return require('sharp');
}

function ensurePillow() {
  const python = join(benchDir, '.venv', 'bin', 'python');
  if (!existsSync(python)) {
    run('python3', ['-m', 'venv', join(benchDir, '.venv')]);
    run(python, ['-m', 'pip', 'install', '--upgrade', 'pip']);
    run(python, ['-m', 'pip', 'install', 'pillow>=11.3']);
  }
  return python;
}

function parseLines(text) {
  return text
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line.startsWith('{'))
    .map((line) => JSON.parse(line));
}

async function benchSharp(sharp, input) {
  const version = createRequire(join(benchDir, 'package.json'))(
    'sharp/package.json',
  ).version;
  const rows = [];
  for (const format of ['jpeg', 'webp', 'avif']) {
    const encode = () => {
      let pipeline = sharp(input, { failOn: 'none' });
      if (format === 'jpeg') {
        pipeline = pipeline.jpeg({ quality: 75, mozjpeg: true, progressive: true });
      } else if (format === 'webp') {
        pipeline = pipeline.webp({ quality: 75, effort: 4 });
      } else {
        pipeline = pipeline.avif({ quality: 75, effort: 4 });
      }
      return pipeline.toBuffer();
    };
    await encode();
    const runs = [];
    let bytes = 0;
    for (let i = 0; i < RUNS; i += 1) {
      const started = performance.now();
      const out = await encode();
      runs.push(performance.now() - started);
      bytes = out.length;
      writeFileSync(join(outDir, `sharp-${format}`), out);
    }
    rows.push({
      impl: 'sharp',
      version,
      format,
      quality: 75,
      effort: format === 'jpeg' ? null : 4,
      bytes,
      runs_ms: runs.map((ms) => Number(ms.toFixed(3))),
    });
  }
  return rows;
}

await ensurePhoto();
const input = readFileSync(photoPath);

const nasmDir = '/tmp/nasm216/nasm-2.16.03';
const path = existsSync(join(nasmDir, 'nasm'))
  ? `${nasmDir}:${process.env.PATH}`
  : process.env.PATH;

mkdirSync(outDir, { recursive: true });
const minipixOut = run(
  'cargo',
  [
    'run',
    '--release',
    '-p',
    'minipix-core',
    '--example',
    'compare_bench',
    '--',
    photoPath,
    outDir,
  ],
  { cwd: root, env: { ...process.env, PATH: path } },
);

const sharp = ensureSharp();
const sharpRows = await benchSharp(sharp, input);

const python = ensurePillow();
const pillowOut = run(python, [join(benchDir, 'pillow_bench.py'), photoPath, outDir]);

const rows = [...parseLines(minipixOut), ...sharpRows, ...parseLines(pillowOut)];
const encoded = rows
  .filter((row) => !row.error)
  .map((row) => join(outDir, `${row.impl}-${row.format}`));
const psnrOut = run(python, [join(benchDir, 'psnr.py'), photoPath, ...encoded]);
const psnr = new Map(
  psnrOut
    .trim()
    .split('\n')
    .filter(Boolean)
    .map((line) => {
      const [name, score] = line.split('\t');
      return [name, Number(score)];
    }),
);
for (const row of rows) {
  if (row.runs_ms) row.median_ms = Number(median(row.runs_ms).toFixed(1));
  const score = psnr.get(`${row.impl}-${row.format}`);
  if (score !== undefined && !Number.isNaN(score)) row.psnr = score;
  console.log(JSON.stringify(row));
}
