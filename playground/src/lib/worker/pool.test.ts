/**
 * pool.test.ts — vitest unit tests for WorkerPool.
 *
 * Strategy:
 *  - FakeWorker captures postMessage calls and exposes `emit()` to
 *    simulate messages back to the pool.
 *  - vi.stubGlobal patches Worker + navigator before each test.
 *  - The pool takes an optional workerFactory for full control.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import type { WorkerResponse } from './protocol';

// ── FakeWorker ────────────────────────────────────────────────────────────────

type MessageHandler = (ev: { data: WorkerResponse }) => void;
type ErrorHandler = (ev: { message: string }) => void;

class FakeWorker {
  onmessage: MessageHandler | null = null;
  onerror: ErrorHandler | null = null;

  /** Messages the pool sent to this worker. */
  sent: unknown[] = [];
  terminated = false;

  postMessage(data: unknown): void {
    this.sent.push(data);
  }

  terminate(): void {
    this.terminated = true;
    this.onmessage = null;
    this.onerror = null;
  }

  /** Simulate a message FROM the worker to the pool. */
  emit(msg: WorkerResponse): void {
    this.onmessage?.({ data: msg });
  }

  /** Simulate a worker-level error. */
  emitError(message: string): void {
    this.onerror?.({ message });
  }
}

// ── helpers ───────────────────────────────────────────────────────────────────

function makeReq(id: number): import('./protocol').CompressRequest {
  return {
    id,
    kind: 'compress',
    data: new ArrayBuffer(8),
    fileName: `file-${String(id)}.jpg`,
    options: {},
  };
}

function makeSuccess(id: number): import('./protocol').SuccessResponse {
  return {
    id,
    ok: true,
    data: new ArrayBuffer(4),
    format: 'jpeg',
    width: 100,
    height: 100,
    bytesIn: 8,
    bytesOut: 4,
    ratio: 0.5,
  };
}

// ── tests ─────────────────────────────────────────────────────────────────────

describe('WorkerPool', () => {
  let workers: FakeWorker[];

  beforeEach(() => {
    workers = [];
    vi.stubGlobal('navigator', { hardwareConcurrency: 4 });
    // Stub Worker so the module-level import doesn't throw in Node.
    // The pool always uses the injected factory; this stub is never called.
    vi.stubGlobal('Worker', FakeWorker);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.resetModules();
  });

  async function createPool(size = 2): Promise<import('./pool').WorkerPool> {
    const { WorkerPool } = await import('./pool');
    return new WorkerPool(() => {
      const w = new FakeWorker();
      workers.push(w);
      return w as unknown as Worker;
    }, size);
  }

  // ── (a) FIFO dispatch with POOL_SIZE concurrency ──────────────────────────

  it('(a) dispatches up to pool-size jobs concurrently, queuing the rest', async () => {
    const pool = await createPool(2);

    const p1 = pool.run(makeReq(1));
    const p2 = pool.run(makeReq(2));
    const p3 = pool.run(makeReq(3)); // should queue

    // Two workers should have been spawned, each with one job.
    expect(workers).toHaveLength(2);
    expect(workers[0]?.sent).toHaveLength(1);
    expect(workers[1]?.sent).toHaveLength(1);

    // Third job is still in the queue — no third worker yet.
    expect(workers).toHaveLength(2);

    // Resolve the first two.
    workers[0]?.emit(makeSuccess(1));
    workers[1]?.emit(makeSuccess(2));

    await Promise.all([p1, p2]);

    // One slot is now free; job 3 should have been dispatched.
    const lastSent = (workers[0]?.sent ?? workers[1]?.sent) as unknown[];
    expect(lastSent.length).toBeGreaterThanOrEqual(1);

    // Resolve job 3 (it went to whichever slot freed first).
    const w3 = workers.find((w) => w.sent.length === 2) ?? workers[0];
    w3?.emit(makeSuccess(3));
    await p3;

    pool.destroy();
  });

  // ── (b) success resolves + frees slot for queued job ─────────────────────

  it('(b) success response resolves the promise and frees the slot', async () => {
    const pool = await createPool(1);

    const p1 = pool.run(makeReq(10));
    const p2 = pool.run(makeReq(11)); // queued

    expect(workers).toHaveLength(1);
    expect(workers[0]?.sent).toHaveLength(1);

    const success10 = makeSuccess(10);
    workers[0]?.emit(success10);

    const result = await p1;
    expect(result.id).toBe(10);
    expect(result.bytesIn).toBe(8);

    // Slot freed → job 11 dispatched to the same worker.
    expect(workers[0]?.sent).toHaveLength(2);

    workers[0]?.emit(makeSuccess(11));
    const result2 = await p2;
    expect(result2.id).toBe(11);

    pool.destroy();
  });

  // ── (c) ErrorResponse rejects with WorkerJobError ────────────────────────

  it('(c) ErrorResponse rejects with WorkerJobError carrying the code', async () => {
    const { WorkerJobError } = await import('./pool');
    const pool = await createPool(1);

    const p = pool.run(makeReq(20));

    workers[0]?.emit({
      id: 20,
      ok: false,
      code: 'UnsupportedFormat',
      message: '[UnsupportedFormat] not supported',
    });

    await expect(p).rejects.toBeInstanceOf(WorkerJobError);
    await expect(p).rejects.toMatchObject({ code: 'UnsupportedFormat' });

    pool.destroy();
  });

  // ── (d) InternalPanic → terminate + respawn + queued jobs still process ──

  it('(d) InternalPanic terminates the worker, respawns it, and processes queued jobs', async () => {
    const { WorkerJobError } = await import('./pool');
    const pool = await createPool(1);

    const p1 = pool.run(makeReq(30));
    const p2 = pool.run(makeReq(31)); // queued while slot busy

    const originalWorker = workers[0];

    // Simulate a panic on job 30.
    originalWorker?.emit({
      id: 30,
      ok: false,
      code: 'InternalPanic',
      message: 'wasm trap: unreachable',
    });

    // p1 must reject with InternalPanic.
    await expect(p1).rejects.toBeInstanceOf(WorkerJobError);
    await expect(p1).rejects.toMatchObject({ code: 'InternalPanic' });

    // Original worker was terminated.
    expect(originalWorker?.terminated).toBe(true);

    // A new worker was spawned for job 31.
    expect(workers).toHaveLength(2);
    const newWorker = workers[1];
    expect(newWorker?.sent).toHaveLength(1);

    // Resolve job 31 on the new worker.
    newWorker?.emit(makeSuccess(31));
    const result = await p2;
    expect(result.id).toBe(31);

    pool.destroy();
  });

  // ── (e) progress callbacks delivered ─────────────────────────────────────

  it('(e) progress callbacks are delivered without settling the job', async () => {
    const pool = await createPool(1);

    const phases: string[] = [];
    const p = pool.run(makeReq(40), (phase) => {
      phases.push(phase);
    });

    workers[0]?.emit({ id: 40, progress: 'decoding' });
    workers[0]?.emit({ id: 40, progress: 'encoding' });

    // Job not yet resolved.
    expect(phases).toEqual(['decoding', 'encoding']);

    workers[0]?.emit(makeSuccess(40));
    const result = await p;
    expect(result.id).toBe(40);

    pool.destroy();
  });
});
