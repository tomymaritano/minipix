"""Encode the same file Pillow-side. One JSON object per line.

Quality 75 is Pillow's own scale, not minipix's. WebP method 4 matches
minipix effort 4 (libwebp method). AVIF is omitted when this Pillow build
has no AVIF encoder — the orchestrator records that instead of a fake number.
"""

from __future__ import annotations

import io
import json
import sys
import time
from pathlib import Path

from PIL import Image

RUNS = 3


def encode(data: bytes, fmt: str) -> tuple[bytes, float]:
    started = time.perf_counter()
    image = Image.open(io.BytesIO(data))
    image.load()
    out = io.BytesIO()
    if fmt == "jpeg":
        image.convert("RGB").save(
            out, format="JPEG", quality=75, progressive=True, optimize=True
        )
    elif fmt == "webp":
        image.save(out, format="WEBP", quality=75, method=4)
    elif fmt == "avif":
        image.save(out, format="AVIF", quality=75)
    else:
        raise ValueError(fmt)
    elapsed_ms = (time.perf_counter() - started) * 1000
    return out.getvalue(), elapsed_ms


def main() -> int:
    path = Path(sys.argv[1])
    out_dir = Path(sys.argv[2]) if len(sys.argv) > 2 else None
    data = path.read_bytes()
    formats = ["jpeg", "webp", "avif"]
    for fmt in formats:
        try:
            encode(data, fmt)
        except Exception as err:  # noqa: BLE001 — report and skip this format
            print(
                json.dumps(
                    {
                        "impl": "pillow",
                        "format": fmt,
                        "error": f"{type(err).__name__}: {err}",
                    }
                )
            )
            continue
        runs: list[float] = []
        last = b""
        for _ in range(RUNS):
            last, elapsed = encode(data, fmt)
            runs.append(elapsed)
        if out_dir is not None:
            (out_dir / f"pillow-{fmt}").write_bytes(last)
        print(
            json.dumps(
                {
                    "impl": "pillow",
                    "format": fmt,
                    "quality": 75,
                    "bytes": len(last),
                    "runs_ms": [round(ms, 3) for ms in runs],
                }
            )
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
