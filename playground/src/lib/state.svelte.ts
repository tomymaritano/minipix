import { WorkerPool, WorkerJobError } from './worker/pool';
import type { JobOptions, SuccessResponse } from './worker/protocol';

export type { JobOptions };

export type JobStatus = 'queued' | 'working' | 'done' | 'error';

export interface Job {
  id: number;
  /** Keep the File for previews; the ArrayBuffer sent to the worker is transferred (neutered). */
  file: File;
  status: JobStatus;
  phase?: 'decoding' | 'encoding';
  options: JobOptions;
  result?: SuccessResponse;
  errorCode?: string;
  errorMessage?: string;
  /** True when format=avif and file.size > 2MB — signals slow-encode warning. */
  avifSlowWarning: boolean;
}

export const globalOptions = $state<JobOptions>({ quality: 75, effort: 4 });

// Single shared pool for the lifetime of the app.
const pool = new WorkerPool();

let nextId = 1;

class PlaygroundState {
  jobs = $state<Job[]>([]);

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
    // Snapshot globalOptions at enqueue time.
    const opts: JobOptions = { ...globalOptions };

    for (const file of files) {
      const id = nextId++;
      const avifSlowWarning = opts.format === 'avif' && file.size > 2 * 1024 * 1024;

      const job: Job = {
        id,
        file,
        status: 'queued',
        options: opts,
        avifSlowWarning,
      };

      this.jobs.push(job);
      // Use the reactive proxy from the array (not the raw `job` reference) so that
      // property mutations in #runJob (status, result, etc.) go through Svelte's
      // reactive setter and trigger DOM re-renders.
      void this.#runJob(this.jobs[this.jobs.length - 1]!);
    }
  }

  retry(job: Job, newOptions?: JobOptions): void {
    const opts: JobOptions = newOptions ?? { ...globalOptions };
    const avifSlowWarning = opts.format === 'avif' && job.file.size > 2 * 1024 * 1024;

    job.status = 'queued';
    job.phase = undefined;
    job.result = undefined;
    job.errorCode = undefined;
    job.errorMessage = undefined;
    job.options = opts;
    job.avifSlowWarning = avifSlowWarning;

    void this.#runJob(job);
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

    const kind = job.options.format ? 'convert' : 'compress';

    try {
      const result = await pool.run(
        {
          id: job.id,
          kind,
          data: buffer,
          fileName: job.file.name,
          // Spread to get a plain (non-proxied) object — Svelte 5's reactive proxy
          // cannot be structured-cloned for postMessage transfer.
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
