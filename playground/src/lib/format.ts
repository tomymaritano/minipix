// Helpers de presentación: tamaños, mime, extensiones, metadatos de formato.

export type FormatId = 'png' | 'jpeg' | 'webp' | 'avif';

export const FORMATS: readonly FormatId[] = ['png', 'jpeg', 'webp', 'avif'];

export const FORMAT_LABEL: Record<FormatId, string> = {
  png: 'PNG',
  jpeg: 'JPEG',
  webp: 'WEBP',
  avif: 'AVIF',
};

const MIME: Record<FormatId, string> = {
  png: 'image/png',
  jpeg: 'image/jpeg',
  webp: 'image/webp',
  avif: 'image/avif',
};

const EXT: Record<FormatId, string> = {
  png: 'png',
  jpeg: 'jpg',
  webp: 'webp',
  avif: 'avif',
};

export function mimeFor(fmt: string): string {
  return (MIME as Record<string, string | undefined>)[fmt] ?? 'application/octet-stream';
}

export function extFor(fmt: string): string {
  return (EXT as Record<string, string | undefined>)[fmt] ?? 'bin';
}

/** "dev-black.png" + "webp" → "dev-black.webp" */
export function renameTo(original: string, fmt: string): string {
  const dot = original.lastIndexOf('.');
  const base = dot > 0 ? original.slice(0, dot) : original;
  return `${base}.${extFor(fmt)}`;
}

/** 71_000 → "71 KB", 1_400_000 → "1.4 MB" */
export function fmtBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${Math.round(n / 1024)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

/** ratio 0.519 → "-48.1%" (negativo = ahorro). Positivo si creció. */
export function fmtSavings(ratio: number): string {
  const pct = (1 - ratio) * 100;
  const sign = pct >= 0 ? '-' : '+';
  return `${sign}${Math.abs(pct).toFixed(1)}%`;
}

/** Códigos de error del worker → texto legible. */
export function errorLabel(code: string | undefined): string {
  switch (code) {
    case 'UnsupportedFormat':
      return 'Unsupported format';
    case 'DecodeError':
      return 'Could not decode';
    case 'EncodeError':
      return 'Could not encode';
    case 'InvalidOptions':
      return 'Invalid options';
    case 'LimitExceeded':
      return 'Image too large';
    case 'IccTransform':
      return 'Color profile error';
    case 'InternalPanic':
      return 'Internal error';
    case 'WasmInitError':
      return 'Engine failed to load';
    case 'WorkerSpawnError':
      return 'Worker failed to start';
    case 'ReadError':
      return 'Could not read file';
    default:
      return 'Error';
  }
}
