# App and tray icons

All PNGs here are generated. Do not edit them by hand.

| Path | What |
|---|---|
| `src/app.svg` | Source: the brand mark (accent `#3CF5A8` stroke on a `#121413` rounded tile) |
| `src/tray.svg` | Source: the monochrome tray mark, stroke `currentColor`, no dot |
| `32x32.png`, `128x128.png`, `128x128@2x.png` (256 px) | App icon sizes |
| `icon.png` (512 px) | App icon. Tauri embeds it as the window icon (`generate_context!`, no `bundle.icon` set) |
| `tray/<dark\|light>/<idle\|active\|attention\|problem>-<16\|24\|32>.png` | 24 tray icons. `dark` is for a dark panel (`#1C1F1E`), `light` for a light one (`#E8EBE9`) |

Tray colours (from `docs/design/graphite-mint/TrayIcons.dc.html`): mark stroke `#DCE0DD` (dark) or `#2C3230` (light).
The dot has radius 3.2 at (13,13) of 16, with a 1.5 ring in the panel colour. `idle` has no dot.

| State | Dot on dark | Dot on light |
|---|---|---|
| active | `#3CF5A8` | `#14935F` |
| attention | `#E3B66B` | `#B07A1E` |
| problem | `#E8968F` | `#C0453B` |

The tray icons are not used yet. The tray shell is a later task.

## Regenerate

```bash
python3 scripts/gen_icons.py           # write every PNG
python3 scripts/gen_icons.py --check   # exit 1 if a PNG is missing or stale
pytest tests/test_icons.py             # sizes, RGBA, dot colours
```

Needs ImageMagick (`magick`). No other dependency. A second run writes byte-identical files with the same
ImageMagick build; another version can change pixels, so re-run and commit the result.
