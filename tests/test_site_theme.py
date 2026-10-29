"""HUN-57 / HUN-132 theme test for site/index.html: the Graphite Mint neon tokens, pinned.

Pins the design tokens (same values as the dashboard, web/src/tokens.css), dark-only rendering,
colours only via ``var(--...)``, self-hosted fonts, no external asset, readable contrast and the
responsive layout. Content is pinned by test_site_content.py.
"""

from __future__ import annotations

import re
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
SITE = ROOT / "site"
PAGE = SITE / "index.html"
COLOR_TOKENS = {
    "--color-bg": "#121413",
    "--color-input": "#0e100f",
    "--color-border": "#262b29",
    "--color-divider": "#1f2321",
    "--color-control": "#2c3230",
    "--color-dot": "#3a413e",
    "--color-heading": "#eaedea",
    "--color-text": "#dce0dd",
    "--color-nav": "#b5bcb7",
    "--color-secondary": "#abb3ae",
    "--color-muted": "#8c9590",
    "--color-accent": "#3cf5a8",
    "--color-accent-soft": "#3cf5a814",
    "--color-accent-line": "#3cf5a859",
    "--color-link": "#7eddb0",
    "--color-link-hover": "#b2edcf",
}
GLOW_TOKENS = {
    "--glow-text": "0 0 12px #3cf5a859",
    "--glow-box": "0 0 0 1px #3cf5a840, 0 0 18px #3cf5a840",
    "--glow-dot": "0 0 8px #3cf5a8",
    "--glow-line": "drop-shadow(0 0 3px #3cf5a899)",
    "--glow-install": "0 0 0 1px #3cf5a833, 0 0 24px #3cf5a81f",
}
TEXT_TOKENS = (
    "--color-heading", "--color-text", "--color-nav", "--color-secondary",
    "--color-muted", "--color-accent", "--color-link", "--color-link-hover",
)
_RAW_COLOR = re.compile(r"#[0-9a-fA-F]{3,8}\b|\b(?:rgba?|hsla?)\s*\(")


def _html() -> str:
    return PAGE.read_text(encoding="utf-8")


def _style(html: str) -> str:
    match = re.search(r"<style>(.*?)</style>", html, re.DOTALL)
    assert match, "no <style> block"
    return match.group(1)


def _root_block(style: str) -> str:
    match = re.search(r":root\s*\{(.*?)\n  \}", style, re.DOTALL)
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
    for name, value in {**COLOR_TOKENS, **GLOW_TOKENS}.items():
        assert tokens.get(name, "").lower() == value, f"{name}: {tokens.get(name)!r}"


def check_dark_only(html: str) -> None:
    assert "prefers-color-scheme" not in html


def check_no_raw_colors(html: str) -> None:
    style = _style(html)
    rules = style.replace(_root_block(style), "", 1)
    assert not _RAW_COLOR.findall(rules), f"raw colour outside :root: {_RAW_COLOR.findall(rules)}"
    assert not re.search(r'\bstyle\s*=\s*"[^"]*(#[0-9a-f]{3}|rgb)', html, re.IGNORECASE)
    assert not re.search(r'\b(?:fill|stroke|stop-color)\s*=\s*"(?:#|rgb)', html, re.IGNORECASE)


def check_no_external_assets(html: str) -> None:
    assert not re.search(r"<script\b[^>]*\bsrc\s*=", html, re.IGNORECASE)
    assert len(re.findall(r"<script\b", html, re.IGNORECASE)) == 1, "only the Copy script is allowed"
    assert "@import" not in html
    assert not re.search(r"url\(\s*['\"]?(?:https?:|//)", html)
    assert not re.search(r"<link\b[^>]*rel=\"(?:stylesheet|preload|preconnect)\"", html)


def test_graphite_mint_tokens_in_root():
    check_tokens(_html())


def test_dark_only():
    check_dark_only(_html())


def test_colors_only_via_variables():
    check_no_raw_colors(_html())


def test_no_external_assets_or_script():
    check_no_external_assets(_html())


def test_fonts_are_self_hosted_with_fallbacks():
    html = _html()
    style = _style(html)
    urls = re.findall(r"@font-face\s*\{[^}]*url\('([^']+)'\)", style)
    assert len(urls) >= 5, urls
    for url in urls:
        assert url.startswith("fonts/") and ".." not in url, url
        assert (SITE / url).is_file(), url
    assert (SITE / "fonts" / "OFL.txt").is_file()
    tokens = _tokens(html)
    assert tokens["--font-sans"].startswith("'IBM Plex Sans'") and "system-ui" in tokens["--font-sans"]
    assert tokens["--font-mono"].startswith("'IBM Plex Mono'") and "monospace" in tokens["--font-mono"]


def test_every_font_file_is_used():
    used = set(re.findall(r"url\('fonts/([^']+)'\)", _style(_html())))
    shipped = {p.name for p in (SITE / "fonts").glob("*.woff2")}
    assert shipped == used


def test_text_contrast_is_aa_on_the_page_background():
    tokens = _tokens(_html())
    for fg in TEXT_TOKENS:
        assert contrast(tokens[fg][:7], tokens["--color-bg"]) >= 4.5, fg
    # the button label on the accent fill
    assert contrast(tokens["--color-input"], tokens["--color-accent"]) >= 4.5


def test_glow_tokens_are_used_and_switched_off_for_more_contrast():
    style = _style(_html())
    for name in GLOW_TOKENS:
        assert f"var({name})" in style, f"{name} is defined but never used"
    assert re.search(r"@media\s*\(prefers-contrast:\s*more\)\s*\{\s*:root\s*\{[^}]*--glow-text:\s*none", style)


def test_responsive_and_accessible_details():
    html = _html()
    style = _style(html)
    assert '<meta name="viewport"' in html
    assert re.search(r"@media\s*\(max-width:\s*768px\)", style), "no narrow-screen rule"
    assert ":focus-visible" in style
    assert "prefers-reduced-motion" in style
    assert 'class="skip-link"' in html
    assert "overflow-x" in style, "code blocks and tables must scroll inside their box"
    assert re.search(r"img\s*\{[^}]*max-width:\s*100%", style), "screenshots must shrink to the viewport"
    assert 'aria-label="Family"' in html and 'aria-label="Sections"' in html


def test_header_and_aside_sizes():
    tokens = _tokens(_html())
    assert tokens["--header-height"] == "4rem"  # 64 px
    assert tokens["--aside-width"] == "15.5rem"  # 248 px


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


def test_second_inline_script_fails():
    with pytest.raises(AssertionError):
        check_no_external_assets(_html().replace("</body>", "<script>1</script></body>", 1))


def test_remote_font_fails():
    with pytest.raises(AssertionError):
        check_no_external_assets(_html().replace("</style>", "@font-face{src:url(https://x.example/f.woff2)}\n</style>", 1))


def test_changed_token_fails():
    with pytest.raises(AssertionError):
        check_tokens(_html().replace("#3cf5a8;", "#00ff88;", 1))
