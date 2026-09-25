"""PSNR of encoded files against Pillow's decode of the source.

Not a perceptual score. Same decoder for every file, so the number only says
how far each encode landed from that decode. Higher is closer.
"""

from __future__ import annotations

import math
import sys
from pathlib import Path

from PIL import Image


def psnr(reference: bytes, candidate: bytes) -> float:
    if len(reference) != len(candidate) or not reference:
        raise ValueError("pixel buffers differ in length")
    total = 0
    for left, right in zip(reference, candidate, strict=True):
        delta = left - right
        total += delta * delta
    mean = total / len(reference)
    if mean == 0:
        return 99.0
    return 10 * math.log10((255 * 255) / mean)


def main() -> int:
    source_path = Path(sys.argv[1])
    source = Image.open(source_path).convert("RGB")
    reference = source.tobytes()
    for raw in sys.argv[2:]:
        path = Path(raw)
        image = Image.open(path).convert("RGB")
        if image.size != source.size:
            print(f"{path.name}\tSIZE {image.size}")
            continue
        score = psnr(reference, image.tobytes())
        print(f"{path.name}\t{score:.2f}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
