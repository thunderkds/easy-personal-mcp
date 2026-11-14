# TASK_GUIDE — T071: Icon assets — app icon from the brand mark, tray icons (4 states × 3 sizes × dark/light) from one SVG source
**Date**: 2026-11-14
**Complexity Level**: C1
**Risk Level**: Low
**Priority**: P2
**Assigned agent**: common-infrastructure
**Agent guide**: `agents/common-infrastructure.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/common-infrastructure.md`
5. Read `rust/docs/UI_REDESIGN_PLAN.md` (rows G5, G6) and `docs/design/graphite-mint/README.md`
6. Read the design sources: the brand-mark `<svg>` in the sidebar of `docs/design/graphite-mint/Main.dc.html` (viewBox 26), and `TrayIcons.dc.html` (icon SVG, viewBox 16, and the `states` list in its script: dot colours for dark and light, idle = no dot)
7. Read `rust/crates/app/tauri.conf.json`, `rust/crates/app/build.rs`, `rust/crates/app/icons/`, and how `window.rs` loads the icon
8. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder

---

## Requirement

**Original request** (owner, 2026-11-14): "refer to this … for the tray icon and icons needed for the redesign the application"; plan accepted.

**Restated intent**:
> 1. **SVG sources** in `rust/crates/app/icons/src/`: `app.svg` (brand mark, accent `#3CF5A8` stroke on `#121413` rounded tile, so it reads on any desktop) and `tray.svg` (the monochrome 16-unit mark from `TrayIcons.dc.html`, no dot).
> 2. **One generator script** `scripts/gen_icons.py` (stdlib + an existing tool: ImageMagick `magick` is installed; PIL is present; no new dependency recorded anywhere) that writes every PNG below from those two sources plus the state colour table. Re-running it gives byte-identical files (or the script states why not).
> 3. **App icon**: `32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.png` (RGBA, replacing the stub square) in `rust/crates/app/icons/`; the window shows the new icon.
> 4. **Tray icons**: `rust/crates/app/icons/tray/<theme>/<state>-<size>.png` for theme `dark|light`, state `idle|active|attention|problem`, size `16|24|32` (24 files). Mark stroke `#DCE0DD` on dark / `#2C3230` on light; dot r 3.2 at (13,13) with a 1.5 ring in the panel colour; dot colours exactly as the `states` table (dark: `#3CF5A8`, `#E3B66B`, `#E8968F`; light: `#14935F`, `#B07A1E`, `#C0453B`); idle has no dot.
> 5. A short `rust/crates/app/icons/README.md`: what each file is, how to regenerate.

**Out of scope**: using the tray icons (the tray shell is a later Watch task); macOS `.icns` / Windows `.ico`; bundling (`bundle.active` stays false).

### Requirement Fidelity Gate
- [x] Restated intent confirmed by the owner (plan accepted 2026-11-14)
- [x] Every Acceptance Criterion traces to the Requirement
- [ ] Requirement Refs: RS-005 (window) in `rust/docs/SPEC.md` (check)

---

## Dependencies & Reachability
**Depends on**: none
**Entry point**: `scripts/gen_icons.py`; consumer `rust/crates/app/src/window.rs`

## Dependency rule
No new crate, npm or Python package.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | Test (`tests/` pytest or a Rust test, your choice, state why): every expected file exists, has the right pixel size, is RGBA; idle tray PNGs have no pixel near any dot colour; each other state has pixels within ±8 of its dot colour in the lower-right quadrant | 3, 4 |
| 2 | Running `scripts/gen_icons.py` twice leaves `git status` clean the second time | 2 |
| 3 | `cargo build --features window` and the Rust suite pass; `app --self-test` still exits 0 (needs a display; if none, say so) | 3 |
| 4 | Screenshot or `magick montage` sheet of all 24 tray PNGs on dark `#1C1F1E` and light `#E8EBE9` panels, compared by LLM vision with `TrayIcons.dc.html` | 4 |

---

## Evaluation & Acceptance

### Verification Command
```bash
python3 scripts/gen_icons.py && git status --short rust/crates/app/icons && cd rust && cargo test --workspace 2>&1 | tail -5
```

### Evidence (filled by reviewer at Stage 4/5)
| Item | Result | Evidence |
|---|---|---|
| New test(s) cover acceptance criteria | ☐ | |
| Re-run is byte-identical | ☐ | |
| Rust build + suite pass | ☐ | |
| code-review | ☐ | |
| UI: visual regression | ☐ | |
| UI: design-system compliance | ☐ | |
| UI: responsiveness | ☐ N/A | fixed-size assets |

## Demonstration
BEFORE: the stub square. AFTER: the contact sheet of app and tray icons.

## UI / Design Acceptance Criteria

### 1. Visual Regression
| Screen / Component | Verification method | Expected result |
|---|---|---|
| 24 tray PNGs + app icon | contact sheet, LLM vision vs `TrayIcons.dc.html` | same shape, dot place and colours |

### 2. Design-System Compliance
| Criterion | Verification method | Expected result |
|---|---|---|
| Colours | AC 1 pixel test | exact table colours |

### 3. Layout / Responsiveness
N/A — fixed-size raster assets.

---

## Approach
SVG first, then the generator, then the test, then wire `tauri.conf.json`/`window.rs` only as far as the window icon needs. Render at 4× and downscale if 16 px edges look poor; record the choice.

## Edge Case Checklist
- 16 px: stroke must stay visible (≥ 1 px); dot must not touch the tile edge illegibly
- ImageMagick missing on another machine → script exits with a clear message, PNGs stay committed

## Files to Change (Predicted)
`rust/crates/app/icons/**`, `scripts/gen_icons.py`, its test, maybe `rust/crates/app/tauri.conf.json` / `window.rs` (icon path only).

## Files Must NOT Touch
`web/**`, `src/easy_personal/**`, any other Rust behaviour.

## Test Plan
Pixel/size test; double-run check; build; contact sheet.

## Completion Checklist
- [ ] AC 1-4 with pasted output
- [ ] Commits on branch `T071` only; final report with `Status:`
