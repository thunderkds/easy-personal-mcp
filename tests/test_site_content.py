"""HUN-57 drift + safety test for site/index.html (scope: PROJECT_SPEC_SITE.md).

Every fact on the page is asserted against its source of truth at test time: the README
(pitch line, install commands), the tools the MCP server registers and the packaged
connector catalog. Nothing here is a hard-coded copy of those lists.
"""

from __future__ import annotations

import asyncio
import re
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
    "personal-agentic-claude.vercel.app": "",
    "easy-verifier.vercel.app": "",
}
READ_STEPS = ["transcripts + kit reports", "ingest", "local store", "MCP / CLI / dashboard"]
CONNECT_STEPS = ["catalog form", "OS keychain", "claude mcp add-json", "launcher"]
REQUIRED_COMMANDS = [
    "npm --prefix web ci && npm --prefix web run build",
    "uv tool install --reinstall --no-cache .",
    "easy-personal ingest",
    "claude mcp add --scope user easy-personal -- easy-personal mcp",
    "easy-personal dashboard",
    "easy-personal schedule install",
    "easy-personal connector add slack",
    "git pull",
    "cp -p ~/.easy-personal/store.sqlite ~/.easy-personal/store.sqlite.backup",
    "uv tool uninstall easy-personal-mcp",
]
DEFINED_TERMS = ("ingest", "connector", "launcher", "kit part")


class Element:
    def __init__(self, tag: str, attrs: dict[str, str | None], section: str | None):
        self.tag, self.attrs, self.section = tag, attrs, section
        self.parts: list[str] = []

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
    shown = {e.attrs["data-tool"] for e in _in(parser, "tools", "li", "data-tool")}
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


# ---- AC 7: safety ----

_SECRET = re.compile(
    r"\b(?:sk-[A-Za-z0-9]{16,}|gh[pousr]_[A-Za-z0-9]{20,}|AKIA[0-9A-Z]{16}|[A-Za-z0-9+/_-]{40,})\b"
)
_EMAIL = re.compile(r"[\w.+-]+@[\w-]+\.[\w.-]+")
_HOST_PATH = ("/home/", "/Users/", "/workspace", "/tmp/")
_FRAGMENT_TOKEN = re.compile(r"127\.0\.0\.1:\d+/?#\S")
_TEXT_SUFFIXES = {".html", ".svg", ".json", ".css", ".txt"}


def violations(text: str) -> list[str]:
    problems = [f"host path {p!r}" for p in _HOST_PATH if p in text]
    problems += [f"email {m!r}" for m in _EMAIL.findall(text)]
    problems += [f"token-shaped {m!r}" for m in _SECRET.findall(text)]
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
        "https://personal-agentic-claude.vercel.app/",
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
