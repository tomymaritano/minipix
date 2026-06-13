import { WorkerPool, WorkerJobError } from './worker/pool';
import type { JobOptions, SuccessResponse } from './worker/protocol';
import type { FormatId } from './format';

export type { JobOptions };

/** Formato de origen por MIME/extensión, para inicializar el selector. */
function detectFormat(file: File): FormatId {
  const t = file.type;
  if (t === 'image/jpeg') return 'jpeg';
  if (t === 'image/webp') return 'webp';
  if (t === 'image/avif') return 'avif';
  if (t === 'image/png') return 'png';
  const ext = file.name.toLowerCase().split('.').pop();
  if (ext === 'jpg' || ext === 'jpeg') return 'jpeg';
  if (ext === 'webp') return 'webp';
  if (ext === 'avif') return 'avif';
  return 'png';
}

export type JobStatus = 'queued' | 'working' | 'done' | 'error';

export interface Job {
  id: number;
  /** Se conserva el File para previews; el ArrayBuffer enviado al worker se transfiere (queda neutered). */
  file: File;
  status: JobStatus;
  phase?: 'decoding' | 'encoding';
  options: JobOptions;
  result?: SuccessResponse;
  errorCode?: string;
  errorMessage?: string;
  /** True cuando format=avif y file.size > 2MB — aviso de encode lento. */
  avifSlowWarning: boolean;
}

/** Opciones del toolbar. format undefined = mismo formato de origen (compress). */
export const globalOptions = $state<JobOptions>({ quality: 75, lossless: false });

const pool = new WorkerPool();
let nextId = 1;

class PlaygroundState {
  jobs = $state<Job[]>([]);
  activeId = $state<number | null>(null);

  active = $derived.by(() => this.jobs.find((j) => j.id === this.activeId) ?? null);

  totals = $derived.by(() => {
    let bytesIn = 0;
    let bytesOut = 0;
    let doneCount = 0;
    for (const job of this.jobs) {
      if (job.status === 'done' && job.result) {
        bytesIn += job.result.bytesIn;
        bytesOut += job.result.bytesOut;
        doneCount++;
      }
    }
    const percent = bytesIn > 0 ? Math.round((1 - bytesOut / bytesIn) * 100) : 0;
    return { bytesIn, bytesOut, percent, doneCount };
  });

  addFiles(files: File[]): void {
    // En el primer drop, el selector toma el formato de origen (WebP wasm = lossless).
    if (globalOptions.format === undefined && files[0]) {
      const f = detectFormat(files[0]);
      globalOptions.format = f;
      if (f === 'webp') globalOptions.lossless = true;
    }
    const opts: JobOptions = { ...globalOptions };
    let firstNew: number | null = null;

    for (const file of files) {
      const id = nextId++;
      if (firstNew === null) firstNew = id;
      const avifSlowWarning = opts.format === 'avif' && file.size > 2 * 1024 * 1024;

      const job: Job = { id, file, status: 'queued', options: opts, avifSlowWarning };
      this.jobs.push(job);
      // Re-leer del array $state: la referencia cruda NO es el proxy reactivo.
      const proxied = this.jobs.find((j) => j.id === job.id);
      if (proxied) void this.#runJob(proxied);
    }

    // El primer archivo nuevo pasa a ser la imagen activa.
    if (firstNew !== null) this.activeId = firstNew;
  }

  /** Re-comprime un job con opciones nuevas (live editing del toolbar). */
  retry(job: Job, newOptions: JobOptions): void {
    const avifSlowWarning = newOptions.format === 'avif' && job.file.size > 2 * 1024 * 1024;
    job.status = 'queued';
    job.phase = undefined;
    job.result = undefined;
    job.errorCode = undefined;
    job.errorMessage = undefined;
    job.options = { ...newOptions };
    job.avifSlowWarning = avifSlowWarning;
    void this.#runJob(job);
  }

  setActive(id: number): void {
    this.activeId = id;
  }

  /** Índice 1-based de la activa y el total (para "2 / 5"). */
  position = $derived.by(() => {
    if (this.activeId === null) return null;
    const idx = this.jobs.findIndex((j) => j.id === this.activeId);
    return idx < 0 ? null : { index: idx + 1, total: this.jobs.length };
  });

  step(delta: number): void {
    if (this.jobs.length === 0) return;
    const idx = this.jobs.findIndex((j) => j.id === this.activeId);
    const base = idx < 0 ? 0 : idx;
    const next = (base + delta + this.jobs.length) % this.jobs.length;
    const job = this.jobs[next];
    if (job) this.activeId = job.id;
  }

  /** Quita la imagen activa; muestra la siguiente o vuelve al estado vacío. */
  removeActive(): void {
    const idx = this.jobs.findIndex((j) => j.id === this.activeId);
    if (idx < 0) return;
    this.jobs.splice(idx, 1);
    const next = this.jobs[idx] ?? this.jobs[idx - 1] ?? null;
    this.activeId = next ? next.id : null;
  }

  clear(): void {
    this.jobs = [];
    this.activeId = null;
  }

  async #runJob(job: Job): Promise<void> {
    let buffer: ArrayBuffer;
    try {
      buffer = await job.file.arrayBuffer();
    } catch (e) {
      job.status = 'error';
      job.errorCode = 'ReadError';
      job.errorMessage = e instanceof Error ? e.message : 'Failed to read file';
      return;
    }

    job.status = 'working';
    const kind = job.options.format || job.options.resize !== undefined ? 'convert' : 'compress';

    try {
      const result = await pool.run(
        {
          id: job.id,
          kind,
          data: buffer,
          fileName: job.file.name,
          options: { ...job.options },
        },
        (phase) => {
          job.phase = phase;
        },
      );
      job.result = result;
      job.status = 'done';
    } catch (e) {
      job.status = 'error';
      if (e instanceof WorkerJobError) {
        job.errorCode = e.code;
        job.errorMessage = e.message;
      } else {
        job.errorCode = 'UnknownError';
        job.errorMessage = e instanceof Error ? e.message : 'Unknown error';
      }
    }
  }
}

export const playground = new PlaygroundState();
