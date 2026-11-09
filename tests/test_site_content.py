"""HUN-57 / HUN-132 drift + safety test for site/index.html (scope: PROJECT_SPEC_SITE.md).

Every fact on the page is asserted against its source of truth at test time: the README
(pitch line, install commands), the tools the MCP server registers and the packaged
connector catalog. The stat strip and the header version come from the same sources (server
registration, catalog, dashboard views in web/src/App.jsx, pyproject.toml). Nothing here is a
hard-coded copy of those lists.
"""

from __future__ import annotations

import asyncio
import re
import tomllib
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import urlparse

import pytest

from easy_personal.connectors import catalog
from easy_personal.surfaces import mcp

ROOT = Path(__file__).resolve().parent.parent
SITE = ROOT / "site"
PAGE = SITE / "index.html"
README = ROOT / "README.md"
SECTION_IDS = ("hero", "flow", "dashboard", "install", "tools", "connectors", "footer")
_VOID = {"meta", "link", "br", "img", "hr", "input"}
ALLOWED_HOSTS = {
    "github.com": "/thunderkds/easy-personal-mcp",
    "easy-kits.vercel.app": "",
    "easy-memory.vercel.app": "",
    "easy-verifier.vercel.app": "",
    "easy-ui-mcp.vercel.app": "",
}
# The five Easy Kit family members, in the order every family list on the site shows them.
FAMILY_ORDER = [
    ("Easy Kit", "https://easy-kits.vercel.app/"),
    ("Easy Verifier", "https://easy-verifier.vercel.app/"),
    ("Easy UI", "https://easy-ui-mcp.vercel.app/"),
    ("Easy Memory", "https://easy-memory.vercel.app/"),
    ("Easy Personal", "#hero"),
]
APP_JSX = ROOT / "web" / "src" / "App.jsx"
GITHUB = "https://github.com/thunderkds/easy-personal-mcp"
ASIDE_GROUPS = ["// overview", "// getting started", "// reference"]
READ_STEPS = ["transcripts + kit reports", "ingest", "local store", "MCP / CLI / dashboard"]
CONNECT_STEPS = ["catalog form", "OS keychain", "claude mcp add-json", "launcher"]
REQUIRED_COMMANDS = [
    "npm --prefix web ci && npm --prefix web run build",
    "uv tool install --reinstall --no-cache .",
    "easy-personal ingest",
    "claude mcp add --scope user easy-personal -- easy-personal mcp",
    "easy-personal dashboard",
    "easy-personal schedule install",
    "easy-personal connector add slack --workspace company",
    "git pull",
    "cp -p ~/.easy-personal/store.sqlite ~/.easy-personal/store.sqlite.backup",
    "uv tool uninstall easy-personal-mcp",
]
DEFINED_TERMS = ("ingest", "connector", "launcher", "kit part")


class Element:
    def __init__(self, tag: str, attrs: dict[str, str | None], section: str | None):
        self.tag, self.attrs, self.section = tag, attrs, section
        self.parts: list[str] = []
        self.anchors: list[Element] = []

    @property
    def text(self) -> str:
        return "".join(self.parts)


class _Parser(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.elements: list[Element] = []
        self.stack: list[Element] = []
        self.section: str | None = None
        self.section_depth = 0

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag in ("section", "footer", "header") and attrs.get("id"):
            self.section = attrs["id"]
            self.section_depth = len(self.stack)
        el = Element(tag, attrs, self.section)
        self.elements.append(el)
        if tag == "a":
            for parent in self.stack:
                parent.anchors.append(el)
        if tag not in _VOID:
            self.stack.append(el)

    def handle_endtag(self, tag):
        for i in range(len(self.stack) - 1, -1, -1):
            if self.stack[i].tag == tag:
                del self.stack[i:]
                break
        if tag in ("section", "footer", "header") and len(self.stack) == self.section_depth:
            self.section = None

    def handle_data(self, data):
        for el in self.stack:
            el.parts.append(data)


def _page() -> _Parser:
    parser = _Parser()
    parser.feed(PAGE.read_text(encoding="utf-8"))
    return parser


def _in(parser: _Parser, section: str, tag: str | None = None, attr: str | None = None):
    return [
        e
        for e in parser.elements
        if e.section == section
        and (tag is None or e.tag == tag)
        and (attr is None or attr in e.attrs)
    ]


def _norm(text: str) -> str:
    return " ".join(text.split())


def registered_tools() -> set[str]:
    return {t.name for t in asyncio.run(mcp.create_app().list_tools())}


def catalog_entries() -> list[catalog.Entry]:
    return [catalog.load(i) for i in catalog.ids()]


def readme_pitch(readme: str) -> str:
    lines = readme.splitlines()
    start = next(i for i, ln in enumerate(lines) if ln.startswith("# "))
    for ln in lines[start + 1 :]:
        if ln.strip():
            return ln.strip()
    raise AssertionError("README has no pitch line")


# ---- checks (functions, so the negative tests can run them on edited sources) ----


def check_tools(parser: _Parser, registered: set[str]) -> None:
    shown = {e.attrs["data-tool"] for e in _in(parser, "tools", "tr", "data-tool")}
    assert len(registered) >= 10, f"server registers {sorted(registered)}"
    assert not registered - shown, f"tool(s) missing from #tools: {sorted(registered - shown)}"
    assert not shown - registered, f"unknown tool(s) in #tools: {sorted(shown - registered)}"
    codes = {e.text for e in _in(parser, "tools", "code")}
    assert shown <= codes, f"tool rows without a <code> name: {sorted(shown - codes)}"


def check_connectors(parser: _Parser, entries: list[catalog.Entry]) -> None:
    shown = {}
    for e in _in(parser, "connectors", "li", "data-provider"):
        shown[e.attrs["data-provider"]] = (e.attrs.get("data-category"), e.attrs.get("data-claude-ai"))
    expected = {
        e.id: (e.category, "yes" if e.claude_ai_connector else "no") for e in entries
    }
    assert shown == expected
    groups = {
        _norm(h.text) for h in _in(parser, "connectors", "h3", "data-category-title")
    }
    assert groups == {e.category for e in entries}


def check_install(parser: _Parser, readme: str) -> None:
    readme_lines = {ln.strip() for ln in readme.splitlines()}
    blocks = _in(parser, "install", "pre")
    assert blocks, "no command blocks in #install"
    shown = []
    for block in blocks:
        for line in block.text.splitlines():
            line = line.strip()
            if line:
                shown.append(line)
                assert line in readme_lines, f"command line not in README: {line!r}"
    for command in REQUIRED_COMMANDS:
        assert command in shown, f"command missing from #install: {command!r}"


def dashboard_views() -> list[str]:
    """The dashboard's views: one `{ id, label, View }` entry per view in web/src/App.jsx."""
    return re.findall(r"\{ id: '(\w+)', label: '[^']+', View:", APP_JSX.read_text(encoding="utf-8"))


def family_links() -> dict[str, str]:
    """The dashboard's `// family` footer links, name -> URL."""
    block = re.search(r"const FAMILY = \[(.*?)\]", APP_JSX.read_text(encoding="utf-8"), re.DOTALL)
    assert block, "no FAMILY list in web/src/App.jsx"
    return dict(re.findall(r"name: '([^']+)', href: '([^']+)'", block.group(1)))


def project_version() -> str:
    return tomllib.loads((ROOT / "pyproject.toml").read_text(encoding="utf-8"))["project"]["version"]


def _all(parser: _Parser, tag: str, attr: str | None = None) -> list[Element]:
    return [e for e in parser.elements if e.tag == tag and (attr is None or attr in e.attrs)]


def _nav(parser: _Parser, label: str) -> Element:
    return next(e for e in _all(parser, "nav", "aria-label") if e.attrs["aria-label"] == label)


def stat_values(parser: _Parser) -> dict[str, int]:
    return {
        e.attrs["data-stat"]: int(re.match(r"\d+", _norm(e.text)).group())
        for e in _in(parser, "hero", "li", "data-stat")
    }


def check_stats(parser: _Parser, tools: int, connectors: int, views: int) -> None:
    assert stat_values(parser) == {"tools": tools, "connectors": connectors, "views": views}


def check_version_pill(parser: _Parser, version: str) -> None:
    pills = [e for e in _all(parser, "span", "class") if "pill" in e.attrs["class"].split()]
    assert [_norm(p.text) for p in pills] == [f"usage dashboard \u00b7 v{version}"]


def check_pitch(parser: _Parser, readme: str) -> None:
    assert readme_pitch(readme) in _norm(" ".join(e.text for e in _in(parser, "hero", "p")))


# ---- AC 1: sections and pitch ----


def test_sections_in_order():
    ids = [
        e.attrs["id"]
        for e in _page().elements
        if e.tag in ("section", "header", "footer") and e.attrs.get("id")
    ]
    assert ids == list(SECTION_IDS)


def test_pitch_line_verbatim_from_readme():
    check_pitch(_page(), README.read_text(encoding="utf-8"))


def test_hero_links_to_github():
    hrefs = {e.attrs["href"] for e in _in(_page(), "hero", "a", "href")}
    assert "https://github.com/thunderkds/easy-personal-mcp" in hrefs


def test_terms_are_defined_on_the_page():
    terms = {_norm(e.text).lower() for e in _page().elements if "term" in (e.attrs.get("class") or "").split()}
    assert set(DEFINED_TERMS) <= terms, sorted(set(DEFINED_TERMS) - terms)


# ---- HUN-132: family layout ----


def test_header_has_brand_version_pill_and_github_button():
    parser = _page()
    header = next(e for e in _all(parser, "header") if "site-header" in (e.attrs.get("class") or "").split())
    assert "Easy Personal" in _norm(header.text)
    check_version_pill(parser, project_version())
    nav = _nav(parser, "Family")
    hrefs = {a.attrs["href"] for a in nav.anchors}
    assert GITHUB in hrefs


def test_family_nav_marks_this_site_and_links_the_other_four():
    parser = _page()
    links = _nav(parser, "Family").anchors
    current = [a for a in links if a.attrs.get("aria-current") == "page"]
    assert [_norm(a.text) for a in current] == ["Easy Personal"]
    shown = {_norm(a.text): a.attrs["href"] for a in links if a not in current and _norm(a.text) != "GitHub"}
    # every sibling the dashboard footer links is also in the site nav
    assert family_links().items() <= shown.items()


def test_family_nav_lists_all_five_in_order_then_github():
    links = _nav(_page(), "Family").anchors
    assert [(_norm(a.text), a.attrs["href"]) for a in links] == FAMILY_ORDER + [("GitHub", GITHUB)]
    nav_html = re.search(r'<nav class="family"[^>]*>(.*?)</nav>', PAGE.read_text(encoding="utf-8"), re.DOTALL).group(1)
    assert len(re.findall(r'<a [^>]*><span class="dot"></span>Easy ', nav_html)) == 5


def test_footer_lists_all_five_family_members_in_order():
    footer_links = [(_norm(e.text), e.attrs["href"]) for e in _in(_page(), "footer", "a", "href")]
    family = [pair for pair in footer_links if pair[1] in {href for _, href in FAMILY_ORDER}]
    assert family == FAMILY_ORDER


def test_aside_groups_link_to_the_pages_own_sections():
    parser = _page()
    assert len(_all(parser, "aside")) == 1
    labels = [_norm(e.text) for e in _all(parser, "div", "class") if "group-label" in e.attrs["class"].split()]
    assert labels == ASIDE_GROUPS
    links = _nav(parser, "Sections").anchors
    targets = [a.attrs["href"] for a in links]
    assert targets and all(t.startswith("#") for t in targets), targets
    ids = {e.attrs["id"] for e in parser.elements if e.attrs.get("id")}
    assert {t[1:] for t in targets} <= ids, sorted({t[1:] for t in targets} - ids)
    assert len([a for a in links if "aria-current" in a.attrs]) == 1, "exactly one current section"


def test_hero_has_eyebrow_h1_and_primary_github_button():
    parser = _page()
    h1 = _in(parser, "hero", "h1")
    assert [_norm(e.text) for e in h1] == ["Easy Personal"]
    eyebrow = [e for e in _in(parser, "hero", "p", "class") if "eyebrow" in e.attrs["class"].split()]
    assert len(eyebrow) == 1 and _norm(eyebrow[0].text).startswith("> ")
    primary = [e for e in _in(parser, "hero", "a", "class") if "btn-primary" in e.attrs["class"].split()]
    assert [e.attrs["href"] for e in primary] == [GITHUB]


def test_stat_strip_numbers_come_from_real_sources():
    check_stats(_page(), len(registered_tools()), len(catalog.ids()), len(dashboard_views()))


def test_dashboard_view_count_is_read_from_the_app():
    views = dashboard_views()
    assert len(views) >= 5 and {"usage", "workspaces", "agents"} <= set(views)


def test_install_box_commands_are_readme_lines_and_copy_button_works():
    html = PAGE.read_text(encoding="utf-8")
    parser = _page()
    box = [e for e in _in(parser, "hero", "pre", "id") if e.attrs["id"] == "install-cmd"]
    assert len(box) == 1
    readme_lines = {ln.strip() for ln in README.read_text(encoding="utf-8").splitlines()}
    lines = [ln.strip() for ln in box[0].text.splitlines() if ln.strip()]
    assert lines and all(ln in readme_lines for ln in lines), lines
    buttons = [e for e in _in(parser, "hero", "button")]
    assert len(buttons) == 1 and buttons[0].attrs.get("type") == "button"
    assert buttons[0].attrs.get("data-copy") == "install-cmd"
    scripts = re.findall(r"<script>(.*?)</script>", html, re.DOTALL)
    assert len(scripts) == 1 and "navigator.clipboard.writeText(" in scripts[0]
    assert "Copied" in scripts[0] and "2000" in scripts[0]


# ---- negatives for the HUN-132 checks ----


def test_drifted_stat_numbers_fail():
    n_tools, n_conn, n_views = len(registered_tools()), len(catalog.ids()), len(dashboard_views())
    with pytest.raises(AssertionError):
        check_stats(_page(), n_tools + 1, n_conn, n_views)
    with pytest.raises(AssertionError):
        check_stats(_page(), n_tools, n_conn - 1, n_views)
    with pytest.raises(AssertionError):
        check_stats(_page(), n_tools, n_conn, n_views + 1)


def test_wrong_version_fails():
    with pytest.raises(AssertionError):
        check_version_pill(_page(), "9.9.9")


# ---- AC 2: flow diagram ----


def _lane(parser: _Parser, lane: str) -> list[Element]:
    return [e for e in _in(parser, "flow", "li", "data-step") if e.attrs.get("data-lane") == lane]


def test_flow_has_two_labelled_lanes_with_ordered_steps():
    parser = _page()
    labels = {e.attrs["data-lane"]: _norm(e.text) for e in _in(parser, "flow", "h3", "data-lane")}
    assert labels.keys() == {"read", "connect"}
    assert labels["read"].startswith("Read") and labels["connect"].startswith("Connect")
    assert [e.attrs["data-step"] for e in _lane(parser, "read")] == READ_STEPS
    assert [e.attrs["data-step"] for e in _lane(parser, "connect")] == CONNECT_STEPS


def test_every_flow_step_has_a_nonempty_caption():
    html = PAGE.read_text(encoding="utf-8")
    items = re.findall(r'<li[^>]*data-step="[^"]+"[^>]*>(.*?)</li>', html, re.DOTALL)
    assert len(items) == len(READ_STEPS) + len(CONNECT_STEPS)
    for item in items:
        match = re.search(r'<span class="caption">(.*?)</span>', item, re.DOTALL)
        assert match and match.group(1).strip(), f"step without caption: {item[:60]!r}"


def test_flow_is_inline_html_without_a_library_or_image():
    parser = _page()
    assert not [e for e in _in(parser, "flow") if e.tag in ("img", "canvas", "script", "iframe")]


# ---- HUN-201 (FR-067): system diagram in #flow and the quick-start strip ----

QUICKSTART_COMMANDS = [
    "uv tool install --reinstall --no-cache .",
    "easy-personal ingest",
    "claude mcp add --scope user easy-personal -- easy-personal mcp",
    "easy-personal dashboard",
]
_SVG_PAINT_OK = {"none", "currentcolor"}


def _flow_svgs(parser: _Parser) -> list[Element]:
    return _in(parser, "flow", "svg")


def _svg_markup(html: str) -> str:
    match = re.search(r"<section id=\"flow\".*?</section>", html, re.DOTALL)
    assert match, "no #flow section"
    svgs = re.findall(r"<svg\b.*?</svg>", match.group(0), re.DOTALL)
    assert len(svgs) == 1, f"#flow must hold exactly one inline svg, found {len(svgs)}"
    return svgs[0]


def check_svg_colors(svg: str) -> None:
    """Only tokens or currentColor: no hex or rgb() anywhere, no paint attribute with a literal colour."""
    assert not re.search(r"#[0-9a-fA-F]{3,8}\b|\b(?:rgba?|hsla?)\s*\(", svg), "literal colour in the diagram"
    for name, value in re.findall(r"\b(fill|stroke|stop-color|color)\s*=\s*\"([^\"]*)\"", svg):
        ok = value.lower() in _SVG_PAINT_OK or value.startswith("url(#")
        assert ok, f"{name}={value!r}: use a class with var(--color-*) or currentColor"


def test_flow_holds_one_inline_svg_with_an_accessible_name():
    parser = _page()
    svgs = _flow_svgs(parser)
    assert len(svgs) == 1
    svg = svgs[0]
    assert svg.attrs.get("role") == "img"
    ids = {e.attrs["id"]: e for e in _in(parser, "flow", attr="id")}
    for attr, min_len in (("aria-labelledby", 1),):
        refs = (svg.attrs.get(attr) or "").split()
        assert len(refs) >= 2, "name and description must both be referenced"
        for ref in refs:
            assert ref in ids, f"{attr} points to a missing id: {ref}"
            assert len(_norm(ids[ref].text)) >= 20, f"{ref} is too short to be a name or description"
    assert [e.tag for e in (ids[r] for r in svg.attrs["aria-labelledby"].split())] == ["title", "desc"]
    assert not [e for e in _in(parser, "flow") if e.tag in ("script", "foreignobject", "image", "use")]


def test_svg_has_a_node_for_every_read_and_connect_step():
    parser = _page()
    nodes = [e.attrs["data-node"] for e in _in(parser, "flow", "g", "data-node")]
    assert nodes == READ_STEPS + CONNECT_STEPS, nodes
    # the diagram and the text lists name the same steps
    assert nodes == [e.attrs["data-step"] for e in _lane(parser, "read") + _lane(parser, "connect")]


def test_svg_draws_the_your_machine_boundary_and_the_provider_outside_it():
    parser = _page()
    boundary = _in(parser, "flow", "g", "data-boundary")
    assert len(boundary) == 1
    assert "your machine" in _norm(boundary[0].text).lower()
    outside = [e for e in _in(parser, "flow", "g", "data-outside")]
    assert [_norm(e.text) for e in outside] and "provider" in _norm(outside[0].text).lower()


def test_svg_uses_tokens_and_currentcolor_only():
    check_svg_colors(_svg_markup(PAGE.read_text(encoding="utf-8")))


def test_svg_with_a_hex_fill_fails():
    svg = _svg_markup(PAGE.read_text(encoding="utf-8"))
    with pytest.raises(AssertionError):
        check_svg_colors(svg.replace("<rect", '<rect fill="#3cf5a8"', 1))
    with pytest.raises(AssertionError):
        check_svg_colors(svg.replace("<rect", '<rect fill="red"', 1))


def test_flow_animation_stops_for_reduced_motion():
    style = re.search(r"<style>(.*?)</style>", PAGE.read_text(encoding="utf-8"), re.DOTALL).group(1)
    animated = "animation:" in re.sub(r"@media \(prefers-reduced-motion: reduce\)\s*\{.*?\n  \}", "", style, flags=re.DOTALL)
    if animated:
        reduce = re.search(r"@media \(prefers-reduced-motion: reduce\)\s*\{(.*?)\n  \}", style, re.DOTALL)
        assert reduce and "animation: none" in reduce.group(1)


def _quickstart() -> list[str]:
    html = PAGE.read_text(encoding="utf-8")
    match = re.search(r'<section class="quickstart".*?</section>', html, re.DOTALL)
    assert match, "no quick-start strip"
    assert html.index('<section id="hero"') < match.start() < html.index('<section id="flow"')
    return re.findall(r"<li\b[^>]*data-quickstep[^>]*>(.*?)</li>", match.group(0), re.DOTALL)


def check_quickstart(readme: str) -> None:
    items = _quickstart()
    assert len(items) == 4, f"quick-start needs 4 steps, found {len(items)}"
    readme_lines = {ln.strip() for ln in readme.splitlines()}
    shown = []
    for item in items:
        commands = re.findall(r'<code class="cmd">(.*?)</code>', item, re.DOTALL)
        assert len(commands) == 1, "each quick-start step carries exactly one command"
        shown.append(_norm(re.sub(r"<[^>]+>", "", commands[0])).replace("&amp;", "&"))
        assert re.search(r"<svg\b[^>]*aria-hidden=\"true\"", item), "each step has a decorative icon"
    for command in shown:
        assert command in readme_lines, f"quick-start command not in README: {command!r}"
    assert shown == QUICKSTART_COMMANDS


def test_quickstart_strip_follows_the_hero_with_four_readme_commands():
    check_quickstart(README.read_text(encoding="utf-8"))


def test_quickstart_links_to_the_install_guide():
    html = PAGE.read_text(encoding="utf-8")
    block = re.search(r'<section class="quickstart".*?</section>', html, re.DOTALL).group(0)
    assert 'href="#install"' in block


def test_edited_readme_quickstart_command_fails():
    readme = README.read_text(encoding="utf-8").replace("easy-personal dashboard\n", "easy-personal board\n")
    with pytest.raises(AssertionError):
        check_quickstart(readme)


# ---- AC 3-5: tools, connectors, install ----


def test_tools_match_server_registration():
    check_tools(_page(), registered_tools())


def test_connectors_match_catalog():
    check_connectors(_page(), catalog_entries())


def test_install_commands_appear_verbatim_in_readme():
    check_install(_page(), README.read_text(encoding="utf-8"))


def test_install_links_to_runbook():
    hrefs = {e.attrs["href"] for e in _in(_page(), "install", "a", "href")}
    assert "https://github.com/thunderkds/easy-personal-mcp/blob/main/RUNBOOK.md" in hrefs


# ---- AC 6: screenshots ----


def test_dashboard_screenshots():
    images = _in(_page(), "dashboard", "img")
    assert 2 <= len(images) <= 4
    for img in images:
        src = img.attrs["src"]
        assert src and not urlparse(src).scheme and ".." not in src, src
        path = SITE / src
        assert path.suffix in (".png", ".webp") and path.is_file(), src
        assert path.stat().st_size <= 400_000, f"{src}: {path.stat().st_size} bytes"
        assert len(_norm(img.attrs.get("alt") or "")) >= 20, f"alt too short: {src}"


# ---- HUN-249: what the dashboard does (one item per change since 0.4.0) ----

DASHBOARD_CHANGES = ["live", "kit-workspace", "report-folders", "scratch", "saved-reports"]
# README and page use the same four terms for the same things.
DASHBOARD_TERMS = ("session row", "kit stage", "Kit by workspace", "report folder")
# Not built (HUN-244, HUN-245): the list must never describe them.
NOT_BUILT = ("fr-091", "fr-092", "store health", "store-check", "send a command", "send-command")
_LATER_VERSION = re.compile(r"\b0\.(?:[5-9]|[1-9]\d)\.\d+\b")


def _page_from(html: str) -> _Parser:
    parser = _Parser()
    parser.feed(html)
    return parser


def check_dashboard_changes(parser: _Parser) -> None:
    headings = [_norm(e.text) for e in _in(parser, "dashboard", "h3")]
    assert "What the dashboard does" in headings, headings
    items = _in(parser, "dashboard", "li", "data-change")
    assert [e.attrs["data-change"] for e in items] == DASHBOARD_CHANGES
    for item in items:
        text = _norm(item.text)
        sentences = re.findall(r"[.!?](?:\s|$)", text)
        assert 1 <= len(sentences) <= 2, f"{item.attrs['data-change']}: {len(sentences)} sentences"
        lowered = text.lower()
        assert not [w for w in NOT_BUILT if w in lowered], f"{item.attrs['data-change']} names a feature that is not built"
        assert "release" not in lowered and not _LATER_VERSION.search(text), "the list promises a release"
    shown = _norm(" ".join(e.text for e in items))
    for term in DASHBOARD_TERMS:
        assert term in shown, f"term missing from the list: {term!r}"


def test_dashboard_lists_what_it_does_one_item_per_change():
    check_dashboard_changes(_page())


def test_dashboard_list_is_inside_the_existing_section_and_adds_no_section_or_image():
    parser = _page()
    sections = [e.attrs["id"] for e in parser.elements if e.tag in ("section", "header", "footer") and e.attrs.get("id")]
    assert sections == list(SECTION_IDS)
    assert len(_in(parser, "dashboard", "img")) == 4
    assert len(_in(parser, "dashboard", "ul")) == 1


def test_readme_uses_the_same_four_terms_as_the_page():
    readme = _norm(README.read_text(encoding="utf-8"))
    for term in DASHBOARD_TERMS:
        assert term in readme, f"README does not use the page's term: {term!r}"
    assert "Add to" in readme and "Scratch" in readme and "save_report" in readme


def test_page_and_readme_promise_no_version_after_0_4_0():
    page_text = _norm(" ".join(e.text for e in _page().elements if e.tag == "body"))
    assert not _LATER_VERSION.search(page_text)
    assert not _LATER_VERSION.search(README.read_text(encoding="utf-8"))
    assert re.findall(r"\bv\d+\.\d+\.\d+\b", page_text) == ["v0.4.0"]
    assert project_version() == "0.4.0"


def test_a_removed_dashboard_item_fails():
    html = PAGE.read_text(encoding="utf-8")
    cut = re.sub(r'<li data-change="scratch">.*?</li>\s*', "", html, count=1, flags=re.DOTALL)
    assert cut != html
    with pytest.raises(AssertionError):
        check_dashboard_changes(_page_from(cut))


def test_a_dashboard_item_that_names_a_feature_not_built_fails():
    html = PAGE.read_text(encoding="utf-8")
    for extra in ("FR-091", "A store health check.", "A button to send a command to Claude."):
        bad = html.replace('<li data-change="scratch">', f'<li data-change="scratch">{extra} ', 1)
        assert bad != html
        with pytest.raises(AssertionError):
            check_dashboard_changes(_page_from(bad))


def test_a_dashboard_item_of_three_sentences_or_a_release_promise_fails():
    html = PAGE.read_text(encoding="utf-8")
    for extra in ("One more. Another one.", "Coming in the next release."):
        bad = html.replace('<li data-change="scratch">', f'<li data-change="scratch">{extra} ', 1)
        assert bad != html
        with pytest.raises(AssertionError):
            check_dashboard_changes(_page_from(bad))


# ---- AC 7: safety ----

_SECRET = re.compile(
    r"\b(?:sk-[A-Za-z0-9]{16,}|gh[pousr]_[A-Za-z0-9]{20,}|AKIA[0-9A-Z]{16}|[A-Za-z0-9+/_-]{40,})\b"
)
_EMAIL = re.compile(r"[\w.+-]+@[\w-]+\.[\w.-]+")
_HOST_PATH = ("/home/", "/Users/", "/workspace", "/tmp/")
_FRAGMENT_TOKEN = re.compile(r"127\.0\.0\.1:\d+/?#\S")
_TEXT_SUFFIXES = {".html", ".svg", ".json", ".css", ".txt"}


def violations(text: str) -> list[str]:
    bare = re.sub(r"https?://\S+", "", text)  # links are checked by link_violations
    problems = [f"host path {p!r}" for p in _HOST_PATH if p in text]
    problems += [f"email {m!r}" for m in _EMAIL.findall(text)]
    problems += [f"token-shaped {m!r}" for m in _SECRET.findall(bare)]
    problems += [f"dashboard token {m!r}" for m in _FRAGMENT_TOKEN.findall(text)]
    return problems


def link_violations(html: str) -> list[str]:
    problems = []
    if re.search(r"<script\b[^>]*\bsrc\s*=", html, re.IGNORECASE):
        problems.append("<script src>")
    for url in re.findall(r"""(?:href|src)\s*=\s*["'](https?:[^"']+)""", html, re.IGNORECASE):
        parsed = urlparse(url)
        prefix = ALLOWED_HOSTS.get(parsed.netloc)
        if prefix is None or not parsed.path.startswith(prefix):
            problems.append(f"link not allowed: {url}")
    return problems


def test_site_tree_has_no_host_path_email_or_token():
    files = [p for p in SITE.rglob("*") if p.is_file()]
    assert {p.name for p in files} >= {"index.html", "favicon.svg"}
    for path in files:
        if path.suffix in _TEXT_SUFFIXES:
            assert violations(path.read_text(encoding="utf-8")) == [], path.name


def test_links_are_allowlisted_and_no_external_script():
    assert link_violations(PAGE.read_text(encoding="utf-8")) == []


# ---- AC 9: footer ----


def test_footer_links():
    hrefs = {e.attrs["href"] for e in _in(_page(), "footer", "a", "href")}
    assert {
        "https://github.com/thunderkds/easy-personal-mcp",
        "https://github.com/thunderkds/easy-personal-mcp#readme",
        "https://easy-kits.vercel.app/",
        "https://easy-verifier.vercel.app/",
    } <= hrefs


# ---- negatives: the checks bite ----


def test_renamed_tool_fails():
    tools = (registered_tools() - {"warnings"}) | {"warnings_v2"}
    with pytest.raises(AssertionError):
        check_tools(_page(), tools)


def test_extra_tool_fails():
    with pytest.raises(AssertionError):
        check_tools(_page(), registered_tools() | {"extra_tool"})


def test_new_provider_fails():
    extra = catalog.parse("zulip", "category: Messaging\nprovider: Zulip\nclaude_ai_connector: no\n"
                          "command: npx -y zulip-mcp@1.0.0\nfield: ZULIP_TOKEN | Token | secret\n")
    with pytest.raises(AssertionError):
        check_connectors(_page(), [*catalog_entries(), extra])


def test_flipped_claude_ai_flag_fails():
    entries = catalog_entries()
    flipped = [catalog.Entry(**{**e.__dict__, "claude_ai_connector": not e.claude_ai_connector})
               if e.id == "slack" else e for e in entries]
    with pytest.raises(AssertionError):
        check_connectors(_page(), flipped)


def test_edited_readme_command_fails():
    readme = README.read_text(encoding="utf-8").replace("easy-personal schedule install\n", "easy-personal schedule add\n", 1)
    with pytest.raises(AssertionError):
        check_install(_page(), readme)


def test_changed_readme_pitch_fails():
    readme = README.read_text(encoding="utf-8").replace("The Easy Kit plug-in", "An Easy Kit plug-in", 1)
    with pytest.raises(AssertionError):
        check_pitch(_page(), readme)


def test_safety_scan_catches_leaks():
    assert violations("see /home/me/x")
    assert violations("see /tmp/x")
    assert violations("mail a.b@example.com")
    assert violations("ghp_" + "a" * 30)
    assert violations("http://127.0.0.1:5000/#abc")
    assert link_violations('<a href="https://evil.example/">x</a>')
    assert link_violations('<script src="https://x.example/a.js"></script>')
    assert link_violations('<a href="https://github.com/other/repo">x</a>')
