#!/usr/bin/env python3
"""Generate every icon PNG of the desktop app from two SVG sources (T071).

    python3 scripts/gen_icons.py           # write the PNGs
    python3 scripts/gen_icons.py --check   # exit 1 if a PNG is missing or stale

Sources, in rust/crates/app/icons/src/:
  app.svg   the brand mark (viewBox 26): accent stroke on a #121413 rounded tile
  tray.svg  the monochrome mark (viewBox 16), stroke "currentColor", no dot

Outputs: the four app PNGs in rust/crates/app/icons/ and 24 tray PNGs in
rust/crates/app/icons/tray/<theme>/<state>-<size>.png. The mark colour, the dot, its
ring and the colours all come from the tables below (the `states` list of
docs/design/graphite-mint/TrayIcons.dc.html). Nothing is drawn by hand.

Rendering: ImageMagick `magick` (its built-in SVG renderer) draws the SVG at 8x the target
size. Then a Lanczos resize makes the target size. This keeps 16 px edges clean. The PNG
carries no metadata chunk (-strip, exclude-chunk=all) and a fixed compression, so a second
run writes byte-identical files for the same ImageMagick build. A different ImageMagick
version can change pixels; then run the script again and commit the new files.
"""

from __future__ import annotations

import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ICONS = ROOT / "rust" / "crates" / "app" / "icons"
SRC = ICONS / "src"

SUPERSAMPLE = 8
APP_SIZES = {"32x32.png": 32, "128x128.png": 128, "128x128@2x.png": 256, "icon.png": 512}
TRAY_SIZES = (16, 24, 32)
STATES = ("idle", "active", "attention", "problem")

# Per theme: mark stroke, panel colour (the dot's ring), and the dot colour of each state.
# idle has no dot. Values are the TrayIcons.dc.html `states` table, exactly.
THEMES = {
    "dark": {
        "stroke": "#DCE0DD",
        "panel": "#1C1F1E",
        "dots": {"active": "#3CF5A8", "attention": "#E3B66B", "problem": "#E8968F"},
    },
    "light": {
        "stroke": "#2C3230",
        "panel": "#E8EBE9",
        "dots": {"active": "#14935F", "attention": "#B07A1E", "problem": "#C0453B"},
    },
}
DOT = '<circle cx="13" cy="13" r="3.2" fill="{dot}" stroke="{panel}" stroke-width="1.5"/>'

PNG_OPTIONS = [
    "-strip",
    "-define", "png:exclude-chunk=all",
    "-define", "png:color-type=6",
    "-define", "png:compression-level=9",
    "-define", "png:compression-strategy=1",
]  # fmt: skip


def tray_svg(theme: str, state: str) -> str:
    """The tray source with the theme stroke and, unless idle, the state dot."""
    t = THEMES[theme]
    svg = (SRC / "tray.svg").read_text().strip().replace("currentColor", t["stroke"])
    dot = ""
    if state != "idle":
        dot = DOT.format(dot=t["dots"][state], panel=t["panel"])
    return svg.replace("</svg>", dot + "</svg>")


def render(svg: str, size: int, out: Path, work: Path) -> None:
    """Draw `svg` at SUPERSAMPLE x `size`, then shrink to `size` x `size` RGBA PNG."""
    big = size * SUPERSAMPLE
    src = work / "in.svg"
    sized = re.sub(
        r'(<svg\b[^>]*?) width="\d+" height="\d+"',
        rf'\1 width="{big}" height="{big}"',
        svg,
        count=1,
    )
    # The built-in renderer draws no stroke on a shape unless the shape carries `stroke-opacity` as its
    # last attribute (it is not inherited from the root, and an earlier position fails too). So add it, value 1,
    # at the end of every stroked shape.
    shaped = re.sub(
        r"(<(?:rect|circle|path)\b[^>]*?\bstroke=[^>]*?)\s*/>", r'\1 stroke-opacity="1"/>', sized
    )
    src.write_text(shaped)
    out.parent.mkdir(parents=True, exist_ok=True)
    cmd = ["magick", "-background", "none", str(src), "-filter", "Lanczos",
           "-resize", f"{size}x{size}!", *PNG_OPTIONS, f"PNG32:{out}"]  # fmt: skip
    subprocess.run(cmd, check=True, capture_output=True, text=True)


def targets() -> list[tuple[Path, str, int]]:
    """Every (output path, svg text, size) this script owns."""
    app = (SRC / "app.svg").read_text().strip()
    items = [(ICONS / name, app, size) for name, size in APP_SIZES.items()]
    for theme in THEMES:
        for state in STATES:
            for size in TRAY_SIZES:
                path = ICONS / "tray" / theme / f"{state}-{size}.png"
                items.append((path, tray_svg(theme, state), size))
    return items


def main(argv: list[str]) -> int:
    check = "--check" in argv
    if shutil.which("magick") is None:
        print("gen_icons: ImageMagick `magick` not found. Install it to regenerate; "
              "the committed PNGs stay valid.", file=sys.stderr)  # fmt: skip
        return 2
    stale = []
    with tempfile.TemporaryDirectory() as tmp:
        work = Path(tmp)
        for path, svg, size in targets():
            if check:
                fresh = work / "fresh.png"
                render(svg, size, fresh, work)
                if not path.exists() or path.read_bytes() != fresh.read_bytes():
                    stale.append(path)
            else:
                render(svg, size, path, work)
    if check:
        for path in stale:
            print(f"stale: {path.relative_to(ROOT)}", file=sys.stderr)
        return 1 if stale else 0
    print(f"gen_icons: wrote {len(targets())} PNGs under {ICONS.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
