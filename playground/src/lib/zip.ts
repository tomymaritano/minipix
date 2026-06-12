// Zip helper — runs on the main thread (buffers are already in memory).
// If perf ever matters with huge batches, move to a worker.
import { zipSync } from 'fflate';
import type { Job } from './state.svelte.js';

function extFor(format: string): string {
  const map: Record<string, string> = {
    png: 'png',
    jpeg: 'jpg',
    webp: 'webp',
    avif: 'avif',
  };
  return map[format] ?? format;
}

function outName(job: Job): string {
  if (!job.result) return job.file.name;
  const base = job.file.name.replace(/\.[^.]+$/, '');
  return `${base}.${extFor(job.result.format)}`;
}

/** Build and trigger download of a zip containing all done jobs. */
export function downloadAllZip(jobs: Job[]): void {
  const doneJobs = jobs.filter((j) => j.status === 'done' && j.result != null);

  if (doneJobs.length === 0) return;

  // Build file map — deduplicate names with a numeric suffix.
  const nameCount = new Map<string, number>();
  const fileMap: Record<string, Uint8Array> = {};

  for (const job of doneJobs) {
    const result = job.result;
    if (!result) continue;
    const base = outName(job);

    const count = nameCount.get(base) ?? 0;
    nameCount.set(base, count + 1);

    const key = count === 0 ? base : base.replace(/(\.[^.]+)$/, `_${String(count)}$1`);
    fileMap[key] = new Uint8Array(result.data);
  }

  const zipped = zipSync(fileMap);
  const blob = new Blob([zipped], { type: 'application/zip' });
  const url = URL.createObjectURL(blob);

  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = 'minipix.zip';
  anchor.click();

  // Revoke after a brief delay so the download can start.
  setTimeout(() => {
    URL.revokeObjectURL(url);
  }, 10000);
}
