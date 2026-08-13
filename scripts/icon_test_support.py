"""PNG fixtures for app icon audit tests."""

import zlib


def rounded_tile(size: int, radius: int) -> list[int]:
    """Returns alpha values for a full-bleed rounded tile."""
    alpha = []
    for row in range(size):
        for column in range(size):
            inset_row = min(row, size - 1 - row)
            inset_column = min(column, size - 1 - column)
            if inset_row >= radius or inset_column >= radius:
                alpha.append(255)
                continue
            distance = (radius - inset_row) ** 2 + (radius - inset_column) ** 2
            alpha.append(255 if distance <= radius**2 else 0)
    return alpha


def bare_mark(size: int, margin: int, thickness: int = 6) -> list[int]:
    """Returns alpha values for an unframed diagonal mark."""
    span = size - 2 * margin
    alpha = []
    for row in range(size):
        for column in range(size):
            inset_row = row - margin
            inset_column = column - margin
            inside = 0 <= inset_row < span and 0 <= inset_column < span
            on_stroke = (
                abs(inset_column - inset_row) <= thickness
                or abs(inset_column + inset_row - (span - 1)) <= thickness
            )
            alpha.append(255 if inside and on_stroke else 0)
    return alpha


def rgba_png(width: int, height: int, alpha: list[int]) -> bytes:
    """Creates a minimal RGBA PNG with the supplied alpha channel."""
    raster = bytearray()
    for row in range(height):
        raster.append(0)
        for column in range(width):
            raster.extend((0, 0, 0, alpha[row * width + column]))
    header = width.to_bytes(4, "big") + height.to_bytes(4, "big") + bytes([8, 6, 0, 0, 0])
    return (
        b"\x89PNG\r\n\x1a\n"
        + png_chunk(b"IHDR", header)
        + png_chunk(b"IDAT", zlib.compress(bytes(raster)))
        + png_chunk(b"IEND", b"")
    )


def png_chunk(kind: bytes, body: bytes) -> bytes:
    """Encodes one PNG chunk."""
    return (
        len(body).to_bytes(4, "big")
        + kind
        + body
        + zlib.crc32(kind + body).to_bytes(4, "big")
    )
