# Graphite Mint design sources (owner, 2026-11-14)

Copied unchanged from the owner's canvas "Easy Personal dashboard — Graphite Mint"
(`https://claude.ai/artifact/GGoKuwVrYPQAVGcT6e7Ptq`, version 1794671911-6efe) so agents can read them.
Plan: `rust/docs/UI_REDESIGN_PLAN.md`.

Each `.dc.html` is one artboard: plain HTML with inline styles inside `<x-dc>`; `{{name}}` holes are filled by the
`renderVals()` script at the bottom (accent `#3CF5A8`, glow on). They do not run outside the canvas; read them as specs.
All numbers, model names and account names in them are sample data.

| File | Artboard | Size |
|---|---|---|
| `Main.dc.html` | Usage (+ right rail) | 1440 wide |
| `Warnings.dc.html` | Warnings & plan | 1440 |
| `Kit.dc.html` | Kit | 1440 |
| `FirstRun.dc.html` | First run (static) | 1440 |
| `Workspaces.dc.html` | Workspaces | 1440 |
| `Agents.dc.html` | Agents | 1440 |
| `Prototype.dc.html` | Clickable first-run flow (state machine in its script) | 1440 |
| `TrayIcons.dc.html` | Tray icon: one icon, four states, 16/24/32 px, dark/light | 1280x640 |
| `TrayPopover.dc.html` | Tray popover (on Linux: a native menu, see the plan) | 360x580 |
