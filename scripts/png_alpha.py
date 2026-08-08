#!/usr/bin/env python3
"""Decode the alpha channel of a PNG using only the Python standard library."""

from __future__ import annotations

import struct
import zlib
from dataclasses import dataclass


PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"
CHANNELS_BY_COLOR_TYPE = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}
ALPHA_CHANNEL_BY_COLOR_TYPE = {4: 1, 6: 3}
PALETTE_COLOR_TYPE = 3
OPAQUE_COLOR_TYPES = (0, 2)
OPAQUE = 255


class PngFormatError(ValueError):
    """Raised when PNG bytes are malformed or use an unsupported encoding."""


@dataclass(frozen=True)
class AlphaRaster:
    """Per-pixel alpha of a decoded PNG, one byte per pixel in row-major order."""

    width: int
    height: int
    alpha: bytes


@dataclass(frozen=True)
class Header:
    """Decoded IHDR fields needed to walk the raster."""

    width: int
    height: int
    bit_depth: int
    color_type: int


def read_alpha_raster(data: bytes) -> AlphaRaster:
    """Returns the per-pixel alpha of a non-interlaced PNG image."""
    header, transparency, pixels = read_chunks(data)
    if header.color_type in OPAQUE_COLOR_TYPES:
        if transparency:
            raise PngFormatError("colour-key transparency is not supported")
        return AlphaRaster(
            header.width, header.height, bytes([OPAQUE]) * (header.width * header.height)
        )
    rows = unfilter_rows(header, pixels)
    return AlphaRaster(header.width, header.height, extract_alpha(header, transparency, rows))


def read_chunks(data: bytes) -> tuple[Header, bytes, bytes]:
    """Returns the header, tRNS bytes, and inflated raster of a PNG image."""
    if not data.startswith(PNG_SIGNATURE):
        raise PngFormatError("bytes do not start with the PNG signature")
    header = None
    transparency = b""
    compressed = bytearray()
    offset = len(PNG_SIGNATURE)
    while offset + 8 <= len(data):
        (length,) = struct.unpack(">I", data[offset : offset + 4])
        kind = data[offset + 4 : offset + 8]
        body = data[offset + 8 : offset + 8 + length]
        if len(body) != length:
            raise PngFormatError(f"chunk {kind.decode('ascii', 'replace')} is truncated")
        offset += length + 12
        if kind == b"IHDR":
            header = read_header(body)
        elif kind == b"tRNS":
            transparency = body
        elif kind == b"IDAT":
            compressed.extend(body)
        elif kind == b"IEND":
            break
    if header is None:
        raise PngFormatError("image has no IHDR chunk")
    if header.color_type in OPAQUE_COLOR_TYPES:
        return header, transparency, b""
    try:
        pixels = zlib.decompress(bytes(compressed))
    except zlib.error as error:
        raise PngFormatError(f"image data cannot be inflated: {error}") from error
    return header, transparency, pixels


def read_header(body: bytes) -> Header:
    """Parses and validates an IHDR chunk body."""
    if len(body) != 13:
        raise PngFormatError("IHDR chunk must be 13 bytes")
    width, height, bit_depth, color_type, compression, filters, interlace = struct.unpack(
        ">IIBBBBB", body
    )
    if width == 0 or height == 0:
        raise PngFormatError("image dimensions must be positive")
    if color_type not in CHANNELS_BY_COLOR_TYPE:
        raise PngFormatError(f"unsupported colour type {color_type}")
    if compression != 0 or filters != 0:
        raise PngFormatError("unsupported compression or filter method")
    if interlace != 0:
        raise PngFormatError("interlaced images are not supported")
    if not valid_bit_depth(color_type, bit_depth):
        raise PngFormatError(f"unsupported bit depth {bit_depth} for colour type {color_type}")
    return Header(width, height, bit_depth, color_type)


def valid_bit_depth(color_type: int, bit_depth: int) -> bool:
    """Reports whether a bit depth is legal for a colour type."""
    if color_type == PALETTE_COLOR_TYPE:
        return bit_depth in (1, 2, 4, 8)
    if color_type == 0:
        return bit_depth in (1, 2, 4, 8, 16)
    return bit_depth in (8, 16)


def unfilter_rows(header: Header, pixels: bytes) -> list[bytes]:
    """Reverses the per-row PNG filters and returns the raw scanlines."""
    bits_per_pixel = CHANNELS_BY_COLOR_TYPE[header.color_type] * header.bit_depth
    stride = (header.width * bits_per_pixel + 7) // 8
    step = max(1, bits_per_pixel // 8)
    if len(pixels) < (stride + 1) * header.height:
        raise PngFormatError("image data is shorter than the declared raster")
    rows = []
    previous = bytes(stride)
    offset = 0
    for _ in range(header.height):
        row = bytearray(pixels[offset + 1 : offset + 1 + stride])
        unfilter_row(pixels[offset], row, previous, step)
        offset += stride + 1
        previous = bytes(row)
        rows.append(previous)
    return rows


def unfilter_row(filter_type: int, row: bytearray, previous: bytes, step: int) -> None:
    """Reverses one scanline filter in place."""
    if filter_type == 0:
        return
    if filter_type == 1:
        for index in range(step, len(row)):
            row[index] = (row[index] + row[index - step]) & 0xFF
        return
    if filter_type == 2:
        for index in range(len(row)):
            row[index] = (row[index] + previous[index]) & 0xFF
        return
    if filter_type == 3:
        for index in range(len(row)):
            left = row[index - step] if index >= step else 0
            row[index] = (row[index] + ((left + previous[index]) >> 1)) & 0xFF
        return
    if filter_type == 4:
        for index in range(len(row)):
            left = row[index - step] if index >= step else 0
            upper_left = previous[index - step] if index >= step else 0
            row[index] = (row[index] + paeth(left, previous[index], upper_left)) & 0xFF
        return
    raise PngFormatError(f"unsupported row filter {filter_type}")


def paeth(left: int, above: int, upper_left: int) -> int:
    """Returns the PNG Paeth predictor for three neighbouring samples."""
    estimate = left + above - upper_left
    to_left = abs(estimate - left)
    to_above = abs(estimate - above)
    to_upper_left = abs(estimate - upper_left)
    if to_left <= to_above and to_left <= to_upper_left:
        return left
    if to_above <= to_upper_left:
        return above
    return upper_left


def extract_alpha(header: Header, transparency: bytes, rows: list[bytes]) -> bytes:
    """Returns one alpha byte per pixel for alpha-bearing colour types."""
    if header.color_type == PALETTE_COLOR_TYPE:
        return palette_alpha(header, transparency, rows)
    channels = CHANNELS_BY_COLOR_TYPE[header.color_type]
    sample_bytes = header.bit_depth // 8
    start = ALPHA_CHANNEL_BY_COLOR_TYPE[header.color_type] * sample_bytes
    stride = channels * sample_bytes
    alpha = bytearray()
    for row in rows:
        alpha.extend(row[start + column * stride] for column in range(header.width))
    return bytes(alpha)


def palette_alpha(header: Header, transparency: bytes, rows: list[bytes]) -> bytes:
    """Returns one alpha byte per pixel for palette images, honouring tRNS."""
    alpha = bytearray()
    for row in rows:
        for index in palette_indices(row, header.bit_depth, header.width):
            alpha.append(transparency[index] if index < len(transparency) else OPAQUE)
    return bytes(alpha)


def palette_indices(row: bytes, bit_depth: int, width: int) -> list[int]:
    """Unpacks packed palette indices from one scanline."""
    if bit_depth == 8:
        return list(row[:width])
    per_byte = 8 // bit_depth
    mask = (1 << bit_depth) - 1
    return [
        (row[column // per_byte] >> (8 - bit_depth * (column % per_byte + 1))) & mask
        for column in range(width)
    ]
