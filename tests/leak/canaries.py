"""Canary helper: unique planted tokens, their encoded forms, and the scan that finds them.

A canary is a random string planted into a fixture field that carries content (message text, tool
input/output, titles, a fake API key, ...). If any canary, in any encoding, shows up in a surface
output, a log or the store, the content boundary (PRD NFR-002 / FR-002) is broken.
"""

import base64
import json
import secrets
from dataclasses import dataclass, field
from pathlib import Path
from urllib.parse import quote

FIXTURE_DIR = Path(__file__).parent.parent / "fixtures" / "leak"
TEMPLATE = "session.jsonl.tmpl"
PLACEHOLDER = "@@{}@@"

# name -> pattern. `{t}` becomes a random hex token, so every run (and every field) is unique.
# Values are chosen to hit the awkward cases: spaces, '@', '/', non-ASCII (stored as \u escapes).
PATTERNS = {
    "api_key": "sk-LEAKCANARY-{t}",
    "email": "leak.{t}@canary.invalid",
    "message_text": "LEAKCANARY priére {t} wire the money",
    "assistant_text": "LEAKCANARY answer {t}",
    "assistant_text_b": "LEAKCANARY answer-b {t}",
    "thinking": "LEAKCANARY thinking {t}",
    "tool_input": "LEAKCANARY tool-input {t}",
    "file_path": "/home/leak/LEAKCANARY_{t}/secret.txt",
    "file_content": "LEAKCANARY file-content {t}",
    "tool_output": "LEAKCANARY tool-output {t}",
    "attachment": "LEAKCANARY attachment {t}",
    "system_text": "LEAKCANARY system {t}",
    "model_text": "LEAKCANARY model name {t}",
    "branch_text": "LEAKCANARY branch name {t}",
    "abstained_text": "LEAKCANARY abstained {t}",
    "abstained_usage": "LEAKCANARY usage {t}",
    "unknown_payload": "LEAKCANARY unknown {t}",
    "last_prompt": "LEAKCANARY last-prompt {t}",
    "custom_title": "LEAKCANARY title {t}",
    "agent_name": "LEAKCANARY agent {t}",
    "malformed": "LEAKCANARY malformed {t}",
    "binary": "LEAKCANARY binary {t}",
}


def _b64_core(raw: bytes) -> list[str]:
    """Base64 of `raw` at all three byte alignments, edges trimmed so each is a stable substring."""
    out = []
    for pad in range(3):
        encoded = base64.b64encode(b"x" * pad + raw).decode()
        for text in (encoded, encoded.replace("+", "-").replace("/", "_")):
            out.append(text[4:-4].rstrip("="))
    return out


def forms_of(value: str) -> dict[str, list[str]]:
    """Every form a value could take once it leaks: raw, JSON/\\u-escaped, URL, base64, hex."""
    raw = value.encode()
    return {
        "raw": [value],
        "json": [json.dumps(value)[1:-1]],
        "unicode-escape": ["".join(f"\\u{ord(c):04x}" for c in value)],
        "url": [quote(value, safe=""), quote(value)],
        "base64": _b64_core(raw),
        "hex": [raw.hex()],
    }


@dataclass
class Canaries:
    values: dict[str, str] = field(default_factory=dict)

    @classmethod
    def new(cls) -> "Canaries":
        return cls({n: p.format(t=secrets.token_hex(8)) for n, p in PATTERNS.items()})

    def render_fixture(self, dest: Path) -> Path:
        """Write the transcript tree (text + binary file) with canaries planted into `dest`."""
        text = (FIXTURE_DIR / TEMPLATE).read_text()
        for name, value in self.values.items():
            # json.dumps escaping keeps the file valid JSON lines whatever the value holds
            text = text.replace(PLACEHOLDER.format(name), json.dumps(value)[1:-1])
        left = [n for n in PATTERNS if PLACEHOLDER.format(n) in text]
        assert not left, f"unplanted placeholders: {left}"
        project = dest / "-work-leak-demo"
        project.mkdir(parents=True)
        (project / "session.jsonl").write_text(text)
        # invalid UTF-8 after a canary: the decoder error must not echo the line anywhere
        (project / "binary.jsonl").write_bytes(
            b'{"type":"user","x":"' + self.values["binary"].encode() + b'\xff\xfe"}\n'
        )
        return dest

    def find(self, blob) -> list[tuple[str, str]]:
        """Return (canary name, form) for every canary found in `blob` (str, bytes or object)."""
        text = _as_text(blob)
        hits = []
        for name, value in self.values.items():
            for form, needles in forms_of(value).items():
                if any(n in text for n in needles):
                    hits.append((name, form))
        return hits

    def assert_clean(self, surface: str, blob) -> None:
        hits = self.find(blob)
        assert not hits, f"LEAK in {surface}: {hits}"


def _as_text(blob) -> str:
    if isinstance(blob, bytes):
        return blob.decode("utf-8", "replace")
    if isinstance(blob, str):
        return blob
    try:
        return json.dumps(blob, default=repr) + "\n" + repr(blob)
    except (TypeError, ValueError):
        return repr(blob)
