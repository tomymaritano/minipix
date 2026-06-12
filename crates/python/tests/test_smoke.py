import hashlib
import json
from pathlib import Path

import minipix
import pytest

ROOT = Path(__file__).resolve().parents[3]
GOLDENS = json.loads((ROOT / "tests/conformance/goldens.json").read_text())


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


@pytest.mark.parametrize("vector", ["gradient_circle", "flat_colors"])
def test_conformance(vector: str) -> None:
    data = (ROOT / f"tests/vectors/{vector}.png").read_bytes()
    out = minipix.compress(data)
    assert sha(out.data) == GOLDENS[f"{vector}.compress.png.q75e4"]
    assert out.ratio > 0 and out.width > 0
    for fmt in ["jpeg", "webp", "avif"]:
        res = minipix.convert(data, format=fmt, effort=4)
        assert sha(res.data) == GOLDENS[f"{vector}.convert.{fmt}.q75e4"], f"{vector} -> {fmt}"


def test_errores_tipados() -> None:
    with pytest.raises(minipix.UnsupportedFormatError):
        minipix.compress(b"garbage")
    with pytest.raises(ValueError):
        minipix.convert(b"garbage", format="bmp")
    with pytest.raises(minipix.LimitExceededError):
        data = (ROOT / "tests/vectors/flat_colors.png").read_bytes()
        minipix.compress(data, max_pixels=10)
