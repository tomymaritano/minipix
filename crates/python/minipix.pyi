# Opciones inválidas / formato desconocido lanzan ValueError (stdlib), no MinipixError.
from typing import Optional

class Output:
    data: bytes
    format: str
    width: int
    height: int
    bytes_in: int
    bytes_out: int
    ratio: float

class MinipixError(Exception): ...
class UnsupportedFormatError(MinipixError): ...
class CodecError(MinipixError): ...
class LimitExceededError(MinipixError): ...

def compress(
    data: bytes,
    *,
    quality: Optional[int] = None,
    effort: Optional[int] = None,
    lossless: Optional[bool] = None,
    alpha_quality: Optional[int] = None,
    jpeg_progressive: Optional[bool] = None,
    max_pixels: Optional[int] = None,
) -> Output: ...

def convert(
    data: bytes,
    *,
    format: str,
    quality: Optional[int] = None,
    effort: Optional[int] = None,
    lossless: Optional[bool] = None,
    alpha_quality: Optional[int] = None,
    jpeg_progressive: Optional[bool] = None,
    max_pixels: Optional[int] = None,
) -> Output: ...
