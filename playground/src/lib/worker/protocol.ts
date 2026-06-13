// Typed worker<->main protocol. Buffers are ALWAYS transferred (not copied).

export interface JobOptions {
  format?: 'png' | 'jpeg' | 'webp' | 'avif';
  quality?: number;
  effort?: number;
  lossless?: boolean;
  /** Si se setea, la imagen se reescala (lado más largo = N px) antes de codificar. */
  resize?: number;
}

export interface CompressRequest {
  id: number;
  kind: 'compress' | 'convert';
  /** Transferred: the ArrayBuffer of the original FILE. */
  data: ArrayBuffer;
  fileName: string;
  options: JobOptions;
}

export interface SuccessResponse {
  id: number;
  ok: true;
  /** Transferred back. */
  data: ArrayBuffer;
  format: string;
  width: number;
  height: number;
  /** ALWAYS the size of the original FILE (not the intermediate RGBA buffer). */
  bytesIn: number;
  bytesOut: number;
  ratio: number;
}

export interface ErrorResponse {
  id: number;
  ok: false;
  code: string;
  message: string;
}

export interface ProgressResponse {
  id: number;
  progress: 'decoding' | 'encoding';
}

export type WorkerResponse = SuccessResponse | ErrorResponse | ProgressResponse;
