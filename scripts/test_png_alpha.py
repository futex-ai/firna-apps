"""Tests for the standard-library PNG alpha decoder."""

import unittest
import zlib

import png_alpha


SIGNATURE = b"\x89PNG\r\n\x1a\n"


class PngAlphaTests(unittest.TestCase):
    def test_rgba_alpha_is_decoded_for_every_row_filter(self) -> None:
        alpha = [0, 64, 128, 255, 200, 12]

        for filter_type in range(5):
            with self.subTest(filter_type=filter_type):
                raster = png_alpha.read_alpha_raster(rgba_png(3, 2, alpha, filter_type))

                self.assertEqual((raster.width, raster.height), (3, 2))
                self.assertEqual(list(raster.alpha), alpha)

    def test_sixteen_bit_rgba_alpha_uses_the_high_byte(self) -> None:
        raster = png_alpha.read_alpha_raster(rgba16_png(2, 1, [0xFFFF, 0x8040]))

        self.assertEqual(list(raster.alpha), [0xFF, 0x80])

    def test_grey_alpha_images_are_decoded(self) -> None:
        raster = png_alpha.read_alpha_raster(grey_alpha_png(2, 2, [0, 255, 128, 32]))

        self.assertEqual(list(raster.alpha), [0, 255, 128, 32])

    def test_palette_alpha_comes_from_the_transparency_chunk(self) -> None:
        raster = png_alpha.read_alpha_raster(palette_png(4, 1, [0, 1, 2, 1], b"\x00\x80"))

        self.assertEqual(list(raster.alpha), [0, 128, 255, 128])

    def test_packed_palette_indices_are_unpacked(self) -> None:
        raster = png_alpha.read_alpha_raster(
            palette_png(4, 1, [0, 1, 1, 0], b"\x00\xff", bit_depth=2)
        )

        self.assertEqual(list(raster.alpha), [0, 255, 255, 0])

    def test_images_without_an_alpha_channel_are_fully_opaque(self) -> None:
        raster = png_alpha.read_alpha_raster(header_only_png(3, 2, bit_depth=8, color_type=2))

        self.assertEqual(list(raster.alpha), [255] * 6)

    def test_colour_key_transparency_is_rejected(self) -> None:
        png = header_only_png(2, 1, bit_depth=8, color_type=2, transparency=b"\x00\x00\x00\x00\x00\x00")

        with self.assertRaises(png_alpha.PngFormatError) as caught:
            png_alpha.read_alpha_raster(png)

        self.assertIn("colour-key transparency", str(caught.exception))

    def test_interlaced_images_are_rejected(self) -> None:
        png = header_only_png(2, 2, bit_depth=8, color_type=6, interlace=1)

        with self.assertRaises(png_alpha.PngFormatError) as caught:
            png_alpha.read_alpha_raster(png)

        self.assertIn("interlaced", str(caught.exception))

    def test_missing_signature_is_rejected(self) -> None:
        with self.assertRaises(png_alpha.PngFormatError) as caught:
            png_alpha.read_alpha_raster(b"GIF89a")

        self.assertIn("PNG signature", str(caught.exception))

    def test_truncated_raster_is_rejected(self) -> None:
        png = build_png(
            ihdr(2, 2, bit_depth=8, color_type=6),
            [(b"IDAT", zlib.compress(b"\x00" * 5))],
        )

        with self.assertRaises(png_alpha.PngFormatError) as caught:
            png_alpha.read_alpha_raster(png)

        self.assertIn("shorter than the declared raster", str(caught.exception))

    def test_unsupported_row_filter_is_rejected(self) -> None:
        png = rgba_png(1, 1, [255], filter_type=0)
        broken = png.replace(zlib.compress(b"\x00\x00\x00\x00\xff"), zlib.compress(b"\x09\x00\x00\x00\xff"))

        with self.assertRaises(png_alpha.PngFormatError) as caught:
            png_alpha.read_alpha_raster(broken)

        self.assertIn("unsupported row filter", str(caught.exception))


def rgba_png(width: int, height: int, alpha: list[int], filter_type: int = 0) -> bytes:
    rows = [
        [byte for column in range(width) for byte in (0, 0, 0, alpha[row * width + column])]
        for row in range(height)
    ]
    return build_png(
        ihdr(width, height, bit_depth=8, color_type=6),
        [(b"IDAT", zlib.compress(filtered(rows, filter_type, step=4)))],
    )


def rgba16_png(width: int, height: int, alpha: list[int]) -> bytes:
    raster = bytearray()
    for row in range(height):
        raster.append(0)
        for column in range(width):
            raster.extend(b"\x00" * 6)
            raster.extend(alpha[row * width + column].to_bytes(2, "big"))
    return build_png(
        ihdr(width, height, bit_depth=16, color_type=6),
        [(b"IDAT", zlib.compress(bytes(raster)))],
    )


def grey_alpha_png(width: int, height: int, alpha: list[int]) -> bytes:
    raster = bytearray()
    for row in range(height):
        raster.append(0)
        for column in range(width):
            raster.extend((0, alpha[row * width + column]))
    return build_png(
        ihdr(width, height, bit_depth=8, color_type=4),
        [(b"IDAT", zlib.compress(bytes(raster)))],
    )


def palette_png(
    width: int, height: int, indices: list[int], transparency: bytes, bit_depth: int = 8
) -> bytes:
    raster = bytearray()
    per_byte = 8 // bit_depth
    for row in range(height):
        raster.append(0)
        packed = bytearray((width + per_byte - 1) // per_byte)
        for column in range(width):
            index = indices[row * width + column]
            shift = 8 - bit_depth * (column % per_byte + 1)
            packed[column // per_byte] |= index << shift
        raster.extend(packed)
    return build_png(
        ihdr(width, height, bit_depth=bit_depth, color_type=3),
        [
            (b"PLTE", b"\x00\x00\x00" * 4),
            (b"tRNS", transparency),
            (b"IDAT", zlib.compress(bytes(raster))),
        ],
    )


def header_only_png(
    width: int,
    height: int,
    bit_depth: int,
    color_type: int,
    interlace: int = 0,
    transparency: bytes = b"",
) -> bytes:
    chunks = []
    if transparency:
        chunks.append((b"tRNS", transparency))
    return build_png(ihdr(width, height, bit_depth, color_type, interlace), chunks)


def filtered(rows: list[list[int]], filter_type: int, step: int) -> bytes:
    raster = bytearray()
    previous = [0] * len(rows[0])
    for row in rows:
        raster.append(filter_type)
        raster.extend(encode_row(row, previous, filter_type, step))
        previous = row
    return bytes(raster)


def encode_row(row: list[int], previous: list[int], filter_type: int, step: int) -> bytes:
    encoded = bytearray()
    for index, value in enumerate(row):
        left = row[index - step] if index >= step else 0
        above = previous[index]
        upper_left = previous[index - step] if index >= step else 0
        if filter_type == 0:
            encoded.append(value)
        elif filter_type == 1:
            encoded.append((value - left) & 0xFF)
        elif filter_type == 2:
            encoded.append((value - above) & 0xFF)
        elif filter_type == 3:
            encoded.append((value - ((left + above) >> 1)) & 0xFF)
        else:
            encoded.append((value - png_alpha.paeth(left, above, upper_left)) & 0xFF)
    return bytes(encoded)


def ihdr(width: int, height: int, bit_depth: int, color_type: int, interlace: int = 0) -> bytes:
    return (
        width.to_bytes(4, "big")
        + height.to_bytes(4, "big")
        + bytes([bit_depth, color_type, 0, 0, interlace])
    )


def build_png(header: bytes, chunks: list[tuple[bytes, bytes]]) -> bytes:
    body = b"".join(chunk(kind, payload) for kind, payload in chunks)
    return SIGNATURE + chunk(b"IHDR", header) + body + chunk(b"IEND", b"")


def chunk(kind: bytes, body: bytes) -> bytes:
    return (
        len(body).to_bytes(4, "big")
        + kind
        + body
        + zlib.crc32(kind + body).to_bytes(4, "big")
    )


if __name__ == "__main__":
    unittest.main()
