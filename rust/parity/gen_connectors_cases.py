"""T046: the catalog-parser oracle. Runs Python's `connectors/catalog.py` (read-only) over a fixed list of hand-written
texts, a seeded set of mutated ones and a seeded set of `shlex.split` strings, and writes what it answered to
`rust/crates/core/tests/connectors_cases.json`, which `connectors_parity.rs` replays against the Rust parser.

    rust/parity/.venv/bin/python rust/parity/gen_connectors_cases.py

Each parse case is `{id, text, want}`; `want` is `{"ok": {...}}`, `{"error": "<CatalogError text>"}` or
`{"crash": "<exception class>"}` (Python raised something that is not a `CatalogError`: Rust refuses with a fixed text).
A shlex case is `{text, want}`; `want` is the token list or `null` for the `ValueError`.
Nothing here reads a keychain, a home folder or the network.
"""

import json
import random
import shlex
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "src"))
from easy_personal.connectors import catalog  # noqa: E402

OUT = ROOT / "rust" / "crates" / "core" / "tests" / "connectors_cases.json"

VALID = (
    "category: Messaging\nprovider: Demo\nclaude_ai_connector: no\n"
    "command: npx -y demo-mcp@1.2.3\nfield: DEMO_TOKEN | Token | secret\n"
)

HAND = [
    ("demo", VALID),
    ("demo", ""),
    ("demo", "# only a comment\n\n   \n"),
    ("Demo", VALID),
    ("", VALID),
    ("a" * 64, VALID),
    ("a" * 65, VALID),
    ("-demo", VALID),
    ("de_mo-1", VALID),
    ("démo", VALID),
    ("demo", VALID.replace("command: npx -y demo-mcp@1.2.3", "command: npx -y demo-mcp")),
    ("demo", VALID.replace("demo-mcp@1.2.3", "@scope/demo-mcp@1.2.3-rc.1+build.5")),
    ("demo", VALID.replace("demo-mcp@1.2.3", "@scope/demo-mcp@1.2.3.4")),
    ("demo", VALID.replace("demo-mcp@1.2.3", "demo-mcp@1.2")),
    ("demo", VALID.replace("demo-mcp@1.2.3", "demo-mcp@1.2.3-")),
    ("demo", VALID.replace("demo-mcp@1.2.3", "Demo-MCP@1.2.3")),
    ("demo", VALID.replace("npx -y demo-mcp@1.2.3", "uvx demo@0.1.0")),
    ("demo", VALID.replace("npx -y demo-mcp@1.2.3", "pipx run demo@0.1.0 --x")),
    ("demo", VALID.replace("npx -y demo-mcp@1.2.3", "pipx run")),
    ("demo", VALID.replace("npx -y demo-mcp@1.2.3", "pipx demo")),
    ("demo", VALID.replace("npx -y demo-mcp@1.2.3", "npx")),
    ("demo", VALID.replace("npx -y demo-mcp@1.2.3", "npx -y -q")),
    ("demo", VALID.replace("npx -y demo-mcp@1.2.3", "python -m demo")),
    ("demo", VALID.replace("npx -y demo-mcp@1.2.3", 'npx -y "demo-mcp@1.2.3" --a "b c"')),
    ("demo", VALID.replace("npx -y demo-mcp@1.2.3", 'npx -y "demo-mcp@1.2.3')),
    ("demo", VALID.replace("npx -y demo-mcp@1.2.3", "npx -y demo-mcp@1.2.3\\")),
    ("demo", VALID.replace("npx -y demo-mcp@1.2.3", "echo '' x")),
    ("demo", VALID + "category: Other\n"),
    ("demo", VALID + "color: red\n"),
    ("demo", VALID + "just a line without a colon\n"),
    ("demo", VALID + ": no key\n"),
    ("demo", VALID.replace("provider: Demo\n", "")),
    ("demo", VALID.replace("provider: Demo\n", "provider:\n")),
    ("demo", VALID.replace("provider: Demo\n", "").replace("category: Messaging\n", "")),
    ("demo", VALID.replace("field: DEMO_TOKEN | Token | secret\n", "")),
    ("demo", VALID.replace("provider: Demo\n", "").replace("field: DEMO_TOKEN | Token | secret\n", "")),
    ("demo", VALID.replace("claude_ai_connector: no", "claude_ai_connector: maybe")),
    ("demo", VALID.replace("claude_ai_connector: no", "claude_ai_connector: yes")),
    ("demo", VALID.replace("claude_ai_connector: no", "claude_ai_connector: yes\nclaude_ai_link: https://claude.ai/x")),
    ("demo", VALID + "claude_ai_link: https://claude.ai/x\n"),
    ("demo", VALID.replace("claude_ai_connector: no", "claude_ai_connector: yes\nclaude_ai_link:")),
    ("demo", VALID.replace("claude_ai_connector: no", "claude_ai_connector: yes\nclaude_ai_link: http://claude.ai/x")),
    ("demo", VALID.replace("claude_ai_connector: no", "claude_ai_connector: yes\nclaude_ai_link: https://")),
    ("demo", VALID.replace("claude_ai_connector: no", "claude_ai_connector: yes\nclaude_ai_link: https://a.b")),
    ("demo", VALID.replace("claude_ai_connector: no", "claude_ai_connector: yes\nclaude_ai_link: https://a.b/")),
    ("demo", VALID.replace("claude_ai_connector: no", "claude_ai_connector: yes\nclaude_ai_link: https://a.b/c d")),
    ("demo", VALID.replace("claude_ai_connector: no", "claude_ai_connector: yes\nclaude_ai_link: https://a.b/c?d=1")),
    ("demo", VALID.replace("claude_ai_connector: no", "claude_ai_connector: yes\nclaude_ai_link: https://a_b/c")),
    ("demo", VALID.replace("claude_ai_connector: no", "claude_ai_connector: yes\nclaude_ai_link: https://a.b:8080/c")),
    ("demo", VALID.replace("claude_ai_connector: no", "claude_ai_connector: yes\nclaude_ai_link: https://a.b/c~d%20e")),
    ("demo", VALID + "field: DEMO_TOKEN | Again | plain\n"),
    ("demo", VALID + "field: OTHER | Label | plain\n"),
    ("demo", VALID + "field: other | Label | plain\n"),
    ("demo", VALID + "field: 1ABC | Label | plain\n"),
    ("demo", VALID + "field: _ABC | Label | plain\n"),
    ("demo", VALID + "field: ABC | | plain\n"),
    ("demo", VALID + "field: ABC | Label\n"),
    ("demo", VALID + "field: ABC | La|bel | plain\n"),
    ("demo", VALID + "field: ABC | Label | Secret\n"),
    ("demo", VALID + "field:\n"),
    ("demo", VALID + "field:ABC|L|plain\n"),
    ("demo", VALID + "   field   :   ABC   |   L   |   plain   \n"),
    ("demo", VALID + "  # indented comment\n"),
    ("demo", VALID + "key with # hash: x\n"),
    ("demo", VALID.replace("\n", "\r\n")),
    ("demo", VALID.replace("\n", "\r")),
    ("demo", VALID.replace("\n", "\x0b")),
    ("demo", VALID.replace("\n", "\x0c")),
    ("demo", VALID.replace("\n", "\x1c")),
    ("demo", VALID.replace("\n", "\x1d")),
    ("demo", VALID.replace("\n", "\x1e")),
    ("demo", VALID.replace("\n", "\x85")),
    ("demo", VALID.replace("\n", " ")),
    ("demo", VALID.replace("\n", " ")),
    ("demo", VALID.replace("\n", "\x1f")),
    ("demo", VALID.replace("\n", "\n\n")),
    ("demo", VALID.rstrip("\n")),
    ("demo", "﻿" + VALID),
    ("demo", VALID.replace("provider: Demo", "provider: Demo ")),
    ("demo", VALID.replace("provider: Demo", "provider:\x1fDemo\x1f")),
    ("demo", VALID.replace("provider: Demo", "provider:​Demo")),
    ("demo", VALID.replace("provider: Demo", "provider: Démo é")),
    ("demo", VALID.replace("provider: Demo", "provider: a: b: c")),
    ("demo", VALID.replace("provider: Demo", "provider: " + "x" * 20000)),
    ("demo", VALID + ("# " + "y" * 20000 + "\n")),
    ("demo", VALID + "\x00\n"),
    ("demo", VALID + "ke\x00y: v\n"),
    ("demo", VALID.replace("category: Messaging\n", "category:    \n")),
    ("demo", VALID.replace("demo-mcp@1.2.3", "demo-mcp@01.02.03")),
    ("demo", VALID.replace("demo-mcp@1.2.3", "demo-mcp@1.2.3+a.b-c")),
    ("demo", VALID.replace("demo-mcp@1.2.3", "@s/s/demo@1.2.3")),
    ("demo", VALID.replace("demo-mcp@1.2.3", "@/demo@1.2.3")),
    ("demo", VALID.replace("demo-mcp@1.2.3", "@demo@1.2.3")),
    ("demo", VALID.replace("demo-mcp@1.2.3", "demo@@1.2.3")),
    ("demo", VALID.replace("demo-mcp@1.2.3", "demo_x.y-z@10.20.30")),
]

FRAGMENTS = [
    "category: Email", "provider: P", "claude_ai_connector: yes", "claude_ai_connector: no",
    "claude_ai_link: https://claude.ai/settings/connectors", "command: npx -y p@1.0.0", "command: uvx p@1.0.0",
    "command: python -m p", "field: A_B | Label | secret", "field: C | Other | plain", "field: A_B | Dup | plain",
    "# comment", "", "   ", "bad line", "category: Two", "field: bad", "command: npx p", "command: npx -y 'p@1.0.0",
    "claude_ai_link: http://x", "claude_ai_link:", "\tprovider:\tTabbed\t",
]

ALPHABET = ["a", "b", "x", " ", " ", "\t", "\n", "\r", "'", '"', "\\", "-", "@", "1", "$", "#", "é"]


def answer(entry_id, text):
    try:
        e = catalog.parse(entry_id, text)
    except catalog.CatalogError as exc:
        return {"error": str(exc)}
    except Exception as exc:  # noqa: BLE001 - recording Python's crash is the point
        return {"crash": type(exc).__name__}
    return {
        "ok": {
            "category": e.category,
            "provider": e.provider,
            "claude_ai_connector": e.claude_ai_connector,
            "claude_ai_link": e.claude_ai_link,
            "command": e.command,
            "fields": [{"env": f.env, "label": f.label, "secret": f.secret} for f in e.fields],
        }
    }


def main():
    rng = random.Random(46)
    cases = [{"id": i, "text": t} for i, t in HAND]
    for _ in range(400):
        lines = [rng.choice(FRAGMENTS) for _ in range(rng.randint(1, 9))]
        if rng.random() < 0.6:
            lines = VALID.splitlines() + lines[: rng.randint(0, 3)]
            rng.shuffle(lines)
        cases.append({"id": "demo", "text": "\n".join(lines) + rng.choice(["", "\n"])})
    for c in cases:
        c["want"] = answer(c["id"], c["text"])
    shlex_cases = []
    for _ in range(3000):
        text = "".join(rng.choice(ALPHABET) for _ in range(rng.randint(0, 14)))
        try:
            want = shlex.split(text)
        except ValueError:
            want = None
        shlex_cases.append({"text": text, "want": want})
    OUT.write_text(json.dumps({"parse": cases, "shlex": shlex_cases}, ensure_ascii=True, sort_keys=True, indent=0) + "\n")
    kinds = {}
    for c in cases:
        k = next(iter(c["want"]))
        kinds[k] = kinds.get(k, 0) + 1
    print(f"parse cases {len(cases)} {kinds}; shlex cases {len(shlex_cases)} "
          f"({sum(1 for c in shlex_cases if c['want'] is None)} ValueError); wrote {OUT}")


main()
