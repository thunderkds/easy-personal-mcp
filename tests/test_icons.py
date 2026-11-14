"""T071: the icon PNGs written by scripts/gen_icons.py have the right files, sizes, alpha and dot colours.

A pytest, not a Rust test: the generator is a Python script and the PNGs are its output. The PNG reader below
is stdlib only (zlib + struct), so the test adds no dependency (PIL is not a project dependency).
"""

from __future__ import annotations

import struct
import zlib
from pathlib import Path

import pytest

ICONS = Path(__file__).resolve().parent.parent / "rust" / "crates" / "app" / "icons"
STATES = ("idle", "active", "attention", "problem")
SIZES = (16, 24, 32)
APP = {"32x32.png": 32, "128x128.png": 128, "128x128@2x.png": 256, "icon.png": 512}
DOTS = {
    "dark": {"active": "#3CF5A8", "attention": "#E3B66B", "problem": "#E8968F"},
    "light": {"active": "#14935F", "attention": "#B07A1E", "problem": "#C0453B"},
}
TOLERANCE = 8


def rgb(hex_colour: str) -> tuple[int, int, int]:
    return tuple(int(hex_colour[i : i + 2], 16) for i in (1, 3, 5))  # type: ignore[return-value]


def read_png(path: Path) -> tuple[int, int, int, int, list[tuple[int, int, int, int]]]:
    """Return (width, height, bit depth, colour type, RGBA pixels) of a non-interlaced 8-bit PNG."""
    data = path.read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n", path
    pos, idat = 8, b""
    width = height = depth = ctype = 0
    while pos < len(data):
        (length,) = struct.unpack(">I", data[pos : pos + 4])
        kind = data[pos + 4 : pos + 8]
        body = data[pos + 8 : pos + 8 + length]
        if kind == b"IHDR":
            width, height, depth, ctype, _, _, interlace = struct.unpack(">IIBBBBB", body)
            assert interlace == 0, path
        elif kind == b"IDAT":
            idat += body
        pos += 12 + length
    if depth != 8 or ctype != 6:
        return width, height, depth, ctype, []
    raw, stride, prev = zlib.decompress(idat), width * 4, bytearray(width * 4)
    pixels: list[tuple[int, int, int, int]] = []
    for row in range(height):
        ftype = raw[row * (stride + 1)]
        line = bytearray(raw[row * (stride + 1) + 1 : (row + 1) * (stride + 1)])
        for i in range(stride):
            a = line[i - 4] if i >= 4 else 0
            b = prev[i]
            c = prev[i - 4] if i >= 4 else 0
            if ftype == 1:
                line[i] = (line[i] + a) & 255
            elif ftype == 2:
                line[i] = (line[i] + b) & 255
            elif ftype == 3:
                line[i] = (line[i] + (a + b) // 2) & 255
            elif ftype == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                pred = a if pa <= pb and pa <= pc else (b if pb <= pc else c)
                line[i] = (line[i] + pred) & 255
        pixels.extend(tuple(line[x : x + 4]) for x in range(0, stride, 4))  # type: ignore[misc]
        prev = line
    return width, height, depth, ctype, pixels


def near(pixel: tuple[int, int, int, int], colour: tuple[int, int, int]) -> bool:
    return pixel[3] > 0 and all(abs(pixel[i] - colour[i]) <= TOLERANCE for i in range(3))


def tray_files() -> list[tuple[str, str, int]]:
    return [(t, s, z) for t in DOTS for s in STATES for z in SIZES]


@pytest.mark.parametrize(("name", "size"), sorted(APP.items()))
def test_app_icon_is_rgba_and_sized(name: str, size: int):
    width, height, depth, ctype, pixels = read_png(ICONS / name)
    assert (width, height) == (size, size)
    assert (depth, ctype) == (8, 6), "8-bit RGBA"
    accent = rgb("#3CF5A8")
    assert any(near(p, accent) for p in pixels), "the accent stroke is drawn"
    assert pixels[0][3] == 0, "the tile corner is transparent (rounded tile)"


@pytest.mark.parametrize(("theme", "state", "size"), tray_files())
def test_tray_icon_size_alpha_and_dot(theme: str, state: str, size: int):
    width, height, depth, ctype, pixels = read_png(ICONS / "tray" / theme / f"{state}-{size}.png")
    assert (width, height) == (size, size)
    assert (depth, ctype) == (8, 6), "8-bit RGBA"
    assert any(p[3] == 0 for p in pixels) and any(p[3] == 255 for p in pixels)
    every_dot = [rgb(c) for c in DOTS[theme].values()]
    if state == "idle":
        assert not any(near(p, c) for p in pixels for c in every_dot), "idle has no dot"
        return
    dot = rgb(DOTS[theme][state])
    lower_right = [
        pixels[y * width + x] for y in range(height // 2, height) for x in range(width // 2, width)
    ]
    assert any(near(p, dot) for p in lower_right), "the state dot sits in the lower-right quadrant"
    upper_left = [pixels[y * width + x] for y in range(height // 2) for x in range(width // 2)]
    assert not any(near(p, dot) for p in upper_left), "the dot is only in the corner"


def test_the_file_set_is_exactly_the_documented_one():
    found = sorted(p.relative_to(ICONS).as_posix() for p in ICONS.rglob("*.png"))
    expected = sorted(list(APP) + [f"tray/{t}/{s}-{z}.png" for t, s, z in tray_files()])
    assert found == expected and len(found) == 28
