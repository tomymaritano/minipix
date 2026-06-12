import init, { compress, convert, encodeRgba } from '../wasm/minipix_wasm.js';
import type { CompressRequest, WorkerResponse } from './protocol';

// Initialize wasm once; awaited before handling any message.
// On failure the error is captured here so handle() can post WasmInitError
// and self.close() — making the slot terminal rather than silently broken.
let initError: Error | null = null;
const ready = init().catch((e: unknown) => {
  initError = e instanceof Error ? e : new Error(String(e));
  // Re-throw so `await ready` in handle() still rejects.
  throw initError;
});

/**
 * Detect AVIF by ISO-BMFF 'ftyp' box at offset 4 + major brand 'avif'/'avis' at 8..11.
 * Matches the core's sniff to avoid routing HEIC/MP4 to browser decoder.
 * Uses explicit undefined-guards for noUncheckedIndexedAccess.
 */
function isAvif(bytes: Uint8Array): boolean {
  if (bytes.length < 12) return false;
  const b4 = bytes[4];
  const b5 = bytes[5];
  const b6 = bytes[6];
  const b7 = bytes[7];
  const b8 = bytes[8];
  const b9 = bytes[9];
  const b10 = bytes[10];
  const b11 = bytes[11];
  return (
    b4 !== undefined &&
    b5 !== undefined &&
    b6 !== undefined &&
    b7 !== undefined &&
    b8 !== undefined &&
    b9 !== undefined &&
    b10 !== undefined &&
    b11 !== undefined &&
    b4 === 0x66 && // 'f'
    b5 === 0x74 && // 't'
    b6 === 0x79 && // 'y'
    b7 === 0x70 && // 'p'
    b8 === 0x61 && // 'a'
    b9 === 0x76 && // 'v'
    b10 === 0x69 && // 'i'
    (b11 === 0x66 || b11 === 0x73) // 'f' (avif) or 's' (avis)
  );
}

async function browserDecodeToRgba(
  data: ArrayBuffer,
): Promise<{ rgba: Uint8Array; width: number; height: number }> {
  const bitmap = await createImageBitmap(new Blob([data]));
  const canvas = new OffscreenCanvas(bitmap.width, bitmap.height);
  const ctx = canvas.getContext('2d');
  if (!ctx) throw new Error('[DecodeError] OffscreenCanvas 2d context unavailable');
  ctx.drawImage(bitmap, 0, 0);
  const img = ctx.getImageData(0, 0, bitmap.width, bitmap.height);
  bitmap.close();
  return { rgba: new Uint8Array(img.data.buffer), width: img.width, height: img.height };
}

self.onmessage = (ev: MessageEvent<CompressRequest>) => {
  void handle(ev.data);
};

async function handle(req: CompressRequest): Promise<void> {
  const post = (msg: WorkerResponse, transfer?: Transferable[]) => {
    self.postMessage(msg, { transfer });
  };

  try {
    await ready;

    // Capture the original file size before we create any Uint8Array view
    // (the buffer may be transferred to wasm after this point).
    const originalSize = req.data.byteLength;
    const bytes = new Uint8Array(req.data);

    let out;

    if (isAvif(bytes)) {
      // AVIF: browser decodes (wasm core has no AV1 decoder — M2 plan).
      post({ id: req.id, progress: 'decoding' });
      const { rgba, width, height } = await browserDecodeToRgba(req.data);
      post({ id: req.id, progress: 'encoding' });
      const target = req.options.format ?? 'avif';
      out = encodeRgba(rgba, width, height, target, req.options);
    } else {
      post({ id: req.id, progress: 'encoding' });
      out = req.kind === 'convert' ? convert(bytes, req.options) : compress(bytes, req.options);
    }

    // Read .data ONCE — each access copies from wasm memory.
    const data = out.data;
    // wasm-bindgen returns a fresh slice copy → the underlying ArrayBuffer is
    // detached from wasm memory and safe to transfer.
    const buf = data.buffer as ArrayBuffer;

    post(
      {
        id: req.id,
        ok: true,
        data: buf,
        format: out.format,
        width: out.width,
        height: out.height,
        // bytesIn/ratio always against the original file, not the RGBA intermediate:
        bytesIn: originalSize,
        bytesOut: out.bytesOut,
        ratio: out.bytesOut / originalSize,
      },
      [buf],
    );
  } catch (e) {
    const m = e instanceof Error ? e.message : String(e);

    // Init failure: the wasm artifact never loaded. This slot is permanently
    // broken — post a typed error and terminate so the pool replaces the slot
    // with clear WasmInitError responses instead of a bricked, silent slot.
    if (
      initError !== null &&
      (e === initError || (e instanceof Error && e.message === initError.message))
    ) {
      post({ id: req.id, ok: false, code: 'WasmInitError', message: m });
      self.close();
      return;
    }

    const code =
      e instanceof WebAssembly.RuntimeError
        ? 'InternalPanic'
        : (/^\[(\w+)\]/.exec(m)?.[1] ?? 'Unknown');

    post({ id: req.id, ok: false, code, message: m });

    if (code === 'InternalPanic') {
      // Wasm instance is dead (trap): close so the pool can respawn.
      self.close();
    }
  }
}
