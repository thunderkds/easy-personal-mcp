"""HUN-57 theme test for site/index.html: the Easy Kit dark neon tokens, pinned.

Pins the design tokens, dark-only rendering, colours only via ``var(--...)``, no external
asset, readable contrast and the responsive layout. Content is pinned by test_site_content.py.
"""

from __future__ import annotations

import re
from pathlib import Path

import pytest

PAGE = Path(__file__).resolve().parent.parent / "site" / "index.html"
KIT_TOKENS = {
    "--bg": "#0a0a12",
    "--surface": "#111128",
    "--surface2": "#16162e",
    "--border": "#1e1e42",
    "--text": "#e2e2f0",
    "--cyan": "#00d4ff",
    "--green": "#00ff88",
    "--amber": "#ffb800",
    "--purple": "#a855f7",
}
DIM_TOKENS = ("--cyan-dim", "--green-dim", "--amber-dim", "--purple-dim")
_RAW_COLOR = re.compile(r"#[0-9a-fA-F]{3,8}\b|\b(?:rgba?|hsla?)\s*\(")


def _html() -> str:
    return PAGE.read_text(encoding="utf-8")


def _style(html: str) -> str:
    match = re.search(r"<style>(.*?)</style>", html, re.DOTALL)
    assert match, "no <style> block"
    return match.group(1)


def _root_block(style: str) -> str:
    match = re.search(r":root\s*\{(.*?)\}", style, re.DOTALL)
    assert match, "no :root block"
    return match.group(1)


def _tokens(html: str) -> dict[str, str]:
    root = _root_block(_style(html))
    return {k: v.strip() for k, v in re.findall(r"(--[\w-]+)\s*:\s*([^;]+);", root)}


def _luminance(hex_color: str) -> float:
    rgb = [int(hex_color[i : i + 2], 16) / 255 for i in (1, 3, 5)]
    lin = [c / 12.92 if c <= 0.03928 else ((c + 0.055) / 1.055) ** 2.4 for c in rgb]
    return 0.2126 * lin[0] + 0.7152 * lin[1] + 0.0722 * lin[2]


def contrast(fg: str, bg: str) -> float:
    a, b = sorted((_luminance(fg), _luminance(bg)), reverse=True)
    return (a + 0.05) / (b + 0.05)


def check_tokens(html: str) -> None:
    tokens = _tokens(html)
    for name, value in KIT_TOKENS.items():
        assert tokens.get(name, "").lower() == value, f"{name}: {tokens.get(name)!r}"
    for name in DIM_TOKENS:
        assert tokens.get(name, "").startswith("rgba("), f"{name} missing"
    assert "--muted" in tokens


def check_dark_only(html: str) -> None:
    assert "prefers-color-scheme" not in html


def check_no_raw_colors(html: str) -> None:
    style = _style(html)
    rules = style.replace(_root_block(style), "", 1)
    assert not _RAW_COLOR.findall(rules), f"raw colour outside :root: {_RAW_COLOR.findall(rules)}"
    assert not re.search(r'\bstyle\s*=\s*"[^"]*(#[0-9a-f]{3}|rgb)', html, re.IGNORECASE)


def check_no_external_assets(html: str) -> None:
    assert not re.search(r"<script\b", html, re.IGNORECASE)
    assert "@import" not in html
    assert not re.search(r"url\(\s*['\"]?https?:", html)
    assert not re.search(r"<link\b[^>]*rel=\"stylesheet\"", html)


def test_kit_tokens_in_root():
    check_tokens(_html())


def test_dark_only():
    check_dark_only(_html())


def test_colors_only_via_variables():
    check_no_raw_colors(_html())


def test_no_external_assets_or_script():
    check_no_external_assets(_html())


def test_system_font_stack():
    style = _style(_html())
    assert "system-ui" in style and "@font-face" not in style


def test_body_and_label_text_contrast_is_aa():
    tokens = _tokens(_html())
    for fg in ("--text", "--muted", "--cyan", "--green", "--amber"):
        for bg in ("--bg", "--surface", "--surface2"):
            assert contrast(tokens[fg], tokens[bg]) >= 4.5, (fg, bg)


def test_responsive_and_accessible_details():
    html = _html()
    style = _style(html)
    assert '<meta name="viewport"' in html
    assert re.search(r"@media\s*\(min-width", style), "no wide-screen layout rule"
    assert ":focus-visible" in style
    assert "prefers-reduced-motion" in style
    assert 'class="skip-link"' in html
    assert "overflow-x" in style, "code blocks must scroll inside their box"
    assert re.search(r"img\s*\{[^}]*max-width:\s*100%", style), "screenshots must shrink to the viewport"


def test_hex_outside_root_fails():
    with pytest.raises(AssertionError):
        check_no_raw_colors(_html().replace("</style>", "  a { color: #fff; }\n</style>", 1))


def test_rgba_outside_root_fails():
    with pytest.raises(AssertionError):
        check_no_raw_colors(_html().replace("</style>", "  a { color: rgba(0,0,0,.5); }\n</style>", 1))


def test_color_scheme_block_fails():
    with pytest.raises(AssertionError):
        check_dark_only(_html().replace("</style>", "@media (prefers-color-scheme: light){}\n</style>", 1))


def test_external_script_fails():
    with pytest.raises(AssertionError):
        check_no_external_assets(_html().replace("</body>", '<script src="https://x.example/a.js"></script></body>', 1))


def test_changed_token_fails():
    with pytest.raises(AssertionError):
        check_tokens(_html().replace("#0a0a12", "#000000", 1))
