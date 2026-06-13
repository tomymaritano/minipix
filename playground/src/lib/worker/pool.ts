import type { CompressRequest, SuccessResponse, WorkerResponse } from './protocol';

export type ProgressCallback = (phase: 'decoding' | 'encoding') => void;

interface PendingJob {
  req: CompressRequest;
  resolve: (r: SuccessResponse) => void;
  reject: (e: Error) => void;
  onProgress?: ProgressCallback;
}

/** Typed error from the worker carrying the protocol error code. */
export class WorkerJobError extends Error {
  constructor(
    public readonly code: string,
    message: string,
  ) {
    super(message);
    this.name = 'WorkerJobError';
  }
}

const defaultWorkerFactory = (): Worker =>
  new Worker(new URL('./compress.worker.ts', import.meta.url), { type: 'module' });

// Keep the pool small: each wasm instance can grow hundreds of MB on large AVIF
// inputs and wasm memory never shrinks. Workers are recycled when they die.
const POOL_SIZE = Math.max(1, Math.min((navigator.hardwareConcurrency || 4) - 1, 3));

export class WorkerPool {
  private readonly size: number;
  /** Live worker per slot index (null = not yet spawned or dead). */
  private readonly slots: (Worker | null)[];
  /**
   * Maps slot index → job id currently running on that slot, or -1 if idle.
   * Stored as a Map to avoid noUncheckedIndexedAccess issues.
   */
  private readonly slotJob: Map<number, number> = new Map();
  private readonly pending: Map<number, PendingJob> = new Map();
  private readonly queue: PendingJob[] = [];
  private readonly factory: () => Worker;

  constructor(workerFactory: () => Worker = defaultWorkerFactory, size: number = POOL_SIZE) {
    this.factory = workerFactory;
    this.size = size;
    this.slots = new Array<Worker | null>(size).fill(null);
    for (let i = 0; i < size; i++) {
      this.slotJob.set(i, -1);
    }
  }

  /**
   * Submit a compression/conversion job.
   * @param req        The message to send (must include a unique req.id).
   * @param onProgress Optional callback for progress phases.
   */
  run(req: CompressRequest, onProgress?: ProgressCallback): Promise<SuccessResponse> {
    return new Promise<SuccessResponse>((resolve, reject) => {
      const job: PendingJob = { req, resolve, reject, onProgress };
      this.dispatch(job);
    });
  }

  /** Terminate all workers (use in tests / cleanup). */
  destroy(): void {
    for (let i = 0; i < this.size; i++) {
      const w = this.slots[i];
      if (w) {
        w.terminate();
        this.slots[i] = null;
        this.slotJob.set(i, -1);
      }
    }
    // Reject all queued and in-flight jobs.
    for (const job of this.queue) {
      job.reject(new WorkerJobError('Destroyed', 'Worker pool destroyed'));
    }
    this.queue.length = 0;
    for (const job of this.pending.values()) {
      job.reject(new WorkerJobError('Destroyed', 'Worker pool destroyed'));
    }
    this.pending.clear();
  }

  // ── internals ──────────────────────────────────────────────────────────────

  private findIdleSlot(): number {
    for (let i = 0; i < this.size; i++) {
      if (this.slotJob.get(i) === -1) return i;
    }
    return -1;
  }

  /** Attempt to send a job immediately, or enqueue it. */
  private dispatch(job: PendingJob): void {
    const slot = this.findIdleSlot();
    if (slot === -1) {
      this.queue.push(job);
      return;
    }
    this.startJob(slot, job);
  }

  /** Dequeue the next pending job into the given free slot, if any. */
  private drainNext(slot: number): void {
    const next = this.queue.shift();
    if (next !== undefined) this.startJob(slot, next);
  }

  private startJob(slot: number, job: PendingJob): void {
    // Lazy-spawn the worker on first use of this slot.
    let worker = this.slots[slot];
    if (!worker) {
      try {
        worker = this.spawnWorker(slot);
      } catch (e) {
        // Factory threw (e.g. 404'd worker script). Settle the job with a
        // typed error, leave the slot null+idle so future dispatches retry.
        const msg = e instanceof Error ? e.message : String(e);
        job.reject(new WorkerJobError('WorkerSpawnError', msg));
        // slotJob stays -1 (idle); drain the next queued job into this slot.
        this.drainNext(slot);
        return;
      }
    }

    this.slotJob.set(slot, job.req.id);
    this.pending.set(job.req.id, job);

    // Transfer the data buffer to avoid a copy.
    worker.postMessage(job.req, [job.req.data]);
  }

  private spawnWorker(slot: number): Worker {
    const worker = this.factory();
    this.slots[slot] = worker;

    worker.onmessage = (ev: MessageEvent<WorkerResponse>) => {
      this.handleMessage(slot, ev.data);
    };

    worker.onerror = (ev: ErrorEvent) => {
      this.handleWorkerError(slot, ev);
    };

    return worker;
  }

  private handleMessage(slot: number, msg: WorkerResponse): void {
    const jobId = this.slotJob.get(slot);
    if (jobId === undefined || jobId === -1) return; // stale message

    // Stale-id guard: defensive against handlers outliving termination.
    // Invariant: no double-settle — worker posts ErrorResponse then self.close();
    // self.close() does NOT fire onerror, and if it ever did, slotJob would be
    // -1 by then (cleared above), so the guard above already returns early.
    if (msg.id !== jobId) return;

    const job = this.pending.get(jobId);
    if (!job) return;

    if ('progress' in msg) {
      // ProgressResponse — call callback and keep the slot busy.
      job.onProgress?.(msg.progress);
      return;
    }

    // Terminal message: free the slot before settling the promise.
    this.pending.delete(jobId);
    this.slotJob.set(slot, -1);

    if (msg.ok) {
      this.drainNext(slot);
      job.resolve(msg);
    } else {
      const { code } = msg;
      const err = new WorkerJobError(code, msg.message);

      if (code === 'InternalPanic' || code === 'WasmInitError') {
        // The worker will self.close() — terminate & respawn proactively.
        // WasmInitError: respawn won't fix a missing artifact but converts the
        // state into clear, typed errors instead of a bricked silent slot.
        this.replaceWorker(slot);
      }

      this.drainNext(slot);
      job.reject(err);
    }
  }

  private handleWorkerError(slot: number, ev: ErrorEvent): void {
    const jobId = this.slotJob.get(slot);

    if (jobId !== undefined && jobId !== -1) {
      const job = this.pending.get(jobId);
      if (job) {
        this.pending.delete(jobId);
        job.reject(new WorkerJobError('WorkerError', ev.message));
      }
    }

    this.slotJob.set(slot, -1);
    this.replaceWorker(slot);
    this.drainNext(slot);
  }

  private replaceWorker(slot: number): void {
    const old = this.slots[slot];
    if (old) {
      old.terminate();
      old.onmessage = null;
      old.onerror = null;
    }
    this.slots[slot] = null;
  }
}
