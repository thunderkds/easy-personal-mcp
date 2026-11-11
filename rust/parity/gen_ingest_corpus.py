"""Differential corpora for the Rust ingest parse layer (T028): Python's answer for every generated string.

Writes two files under `ingest/golden/`, read by `rust/crates/core/tests/ingest_parse_corpus.rs`:

- `corpus-time.json`: `claude_code._utc` (`datetime.fromisoformat` + `astimezone(UTC)` + `strftime`) on hand-picked and
  generated timestamp strings. `why` says `ok`, `naive` (no zone: `_utc` gives None), `invalid` (ValueError) or
  `crash:<class>` (an exception `_utc` does not catch: the Python ingest crashes on such a line; Rust gives None).
- `corpus-label.json`: `_label` (`_LABEL` with the `$` quirk), `_WORD.fullmatch`, `SECRET.search`, `_bash_word` and
  `_host` on generated strings, plus three tables of the Python runtime the Rust code relies on (`str.split`
  whitespace, the characters whose NFKC form holds one of `/?#@:` for `urlsplit`'s netloc check, the characters that
  `str.lower` maps to ASCII).

The functions are imported from the app itself (`easy_personal.sources.claude_code`, read only), so the corpus is
the oracle, not a copy of it. Deterministic (fixed seed, sorted keys); no file is read but the app's modules.

    rust/parity/.venv/bin/python rust/parity/gen_ingest_corpus.py          # (re)writes the two files
    rust/parity/.venv/bin/python rust/parity/gen_ingest_corpus.py --check  # exit 0 equal, 1 different (names the files)
"""

import argparse
import datetime
import json
import platform
import random
import sys
import unicodedata
from pathlib import Path

from easy_personal.core.secretlike import SECRET
from easy_personal.sources import claude_code

GOLDEN = Path(__file__).resolve().parent / "ingest" / "golden"
SEED = 20261111
SURROGATE = "\udc80"


def unique(values):
    seen, out = set(), []
    for value in values:
        if value not in seen:
            seen.add(value)
            out.append(value)
    return out


def mutate(rng, text, pool):
    """One deterministic edit: drop, insert or replace one character."""
    if not text:
        return rng.choice(pool)
    i = rng.randrange(len(text))
    op = rng.randrange(3)
    if op == 0:
        return text[:i] + text[i + 1 :]
    if op == 1:
        return text[:i] + rng.choice(pool) + text[i:]
    return text[:i] + rng.choice(pool) + text[i + 1 :]


# -- time


def time_inputs(rng):
    hand = [  # the odd-case table (ingest/fixtures/synthetic/time/stamps.jsonl) and its neighbours
        "2026-01-10T10:00:10",
        "2026-01-10T10:00:10+07:00",
        "2026-01-10T10:00:10-05:30",
        "2026-01-10T10:00:10+00:00",
        "2026-01-10T10:00:10Z",
        "2026-01-10T10:00:10.1Z",
        "2026-01-10T10:00:10.123Z",
        "2026-01-10T10:00:10.123456Z",
        "2026-01-10T10:00:10.123456789Z",
        "2026-01-10T10:00:10,5Z",
        "20260110T100010Z",
        "2026-01-10 10:00:10Z",
        "2026-01-10T10:00Z",
        "2026-01-10T10Z",
        "2026-W02-6T10:00:10Z",
        "2026-01-10T24:00:00Z",
        "2026-01-10t10:00:10z",
        "2026-01-10T10:00:10z",
        "2026-01-10T23:59:60Z",
        "2026-01-10",
        "",
        "1736503210",
        "2026-12-31T24:00:00Z",
        "2024-02-28T24:00:00Z",
        "2023-02-28T24:00:00Z",
        "9999-12-31T24:00:00Z",
        "0001-01-01T00:00:00Z",
        "0001-01-01T00:00:00+00:01",
        "0001-01-01T00:59:59+01:00",
        "0001-01-01T01:00:00+01:00",
        "9999-12-31T23:59:59-00:01",
        "9999-12-31T23:59:59.999999Z",
        "9999-12-31T22:59:59.999999-01:00",
        "0999-05-01T00:00:00Z",
        "0099-05-01T00:00:00Z",
        "2026-01-10" + SURROGATE + "10:00:10Z",
        "2026-01-10" + SURROGATE + SURROGATE + "10:00:10Z",
        "2026-01-10T10:00:10" + SURROGATE + "Z",
        "2026-01-10\u00e910:00:10Z",
        "2026-01-10T10:00:10.Z",
        "2026-01-10T10:00:10.",
        "2026-01-10T",
        "2026-01-10TZ",
        "2026-01-10T1Z",
        "2026-01-10T1010101Z",
        "2026-01-10T1010101",
        "2026-01-10T10:00:10.1234567x",
        "2026-01-10T10:00:10+05:30:15.1234567",
        "2026-01-10T10:00:10+24:00",
        "2026-01-10T10:00:10+23:59:59.999999",
        "2026-01-10T10:00:10-23:59:59.999999",
        "2026-01-10T10:00:10+05:99",
        "2026-W53-1T00:00:00Z",
        "2020-W53-7T00:00:00Z",
        "2015-W53-5T00:00:00Z",
        "2026-W00-1T00:00:00Z",
        "9999-W52-5T00:00:00Z",
        "9999-W52-6T00:00:00Z",
        "0001-W01-1T00:00:00Z",
        "2026W026T10:00Z",
        "2026W02T10:00Z",
        "2026-W02T10:00Z",
        "2026-W02-T10:00Z",
        "2026W0212:00Z",
        "2026-W02-610:00Z",
        "2026-W02-6-10:00Z",
        "\uff12\uff10\uff12\uff16-01-10T10:00:10Z",
        "2026-01-10T\uff11\uff10:00:10Z",
        "2026-01-10T10:00:10\uff3a",
        " 2026-01-10T10:00:10Z",
        "2026-01-10T10:00:10Z ",
        "2026-01-10T10:00:10Z\n",
        "2026-01-10T10:00:10\n",
        "2026-01-10T10:00:10+01:00\n",
        "2026-01-10T10:00:10\x00Z",
        "2026-01-10T10:00:10Z\x00junk",
        "2026-01-10T10:00:00+00:00:00.5",
        "2026-01-10T10:00:00+05:00:00.5",
        "2026-01-10T10:00:00-05:00:00.5",
        "2026-01-10T10:00:00+00:00:01.5",
        "2026-01-10T10:00:00-00:00:00.000001",
        "2026-01-10T10:00:00+23:99",
        "2026-01-10T10:00:00+99:00",
        "2026-01-10T10:00:00+23:59:60",
        "2026-01-10T10:00:00+00:60",
        "2026-01-10€10:00:00Z",
        "2026-01-10\U0001f600x0:00:00Z",
        "2026-01-10\U0001f60010:00:00Z",
        "2026-W02-é10:00Z",
        "2026-01-10é",
    ]
    dates = [
        "2026-01-10", "20260110", "2026-W02-6", "2026W026", "2026-W02", "2026W02", "2024-02-29", "2023-02-29",
        "2026-12-31", "9999-12-31", "0001-01-01", "0000-01-01", "2026-13-01", "2026-00-10", "2026-01-32",
        "2026-01-00", "2020-W53-1", "2026-W53-1", "2026-W01-0", "2026-W01-8", "1-01-01", "2026-1-10", "2026-01-1",
        "202601-10", "2026-0110", "2026/01/10", "2026-001", "2026001", "2026-W1", "+2026-01-10",
    ]  # fmt: skip
    seps = ["T", " ", "t", "x", SURROGATE, "\u00e9", "1", "-", "_", "\n", ":"]
    times = [
        "10", "10:00", "1000", "10:00:10", "100010", "10:00:10.1", "10:00:10.12", "10:00:10.123", "10:00:10.1234",
        "10:00:10.12345", "10:00:10.123456", "10:00:10.1234567", "10:00:10.123456789", "10:00:10,5", "10:00:10.",
        "24:00", "24:00:00", "24:00:00.000000", "24:00:01", "24:00:00.000001", "23:59:60", "25:00", "10:60",
        "10:00:61", "1", "10:0", "10:00:0", "1010101", "10:1010", "1010:10", "10.5", "10:30.5", "10:00:10:00",
        "10:00:10.12a", "", " 10:00", "00:00", "23:59:59.999999", "100010.5", "100010,25", "10:00:10.0000001",
    ]  # fmt: skip
    tzs = [
        "", "Z", "z", "+00:00", "-00:00", "+07:00", "-05:30", "+0700", "+07", "-0530", "+05:30:15", "+05:30:15.5",
        "+053015.123456", "+24:00", "+23:59", "+23:59:59.999999", "+05:99", "+05:30Z", "Z+01:00", "+", "-", "+1",
        "+123", "ZZ", "+05:30:15.1234567", "+05:30:15.", "UTC", " Z", "Z ", "+05:30:60", "+05:60", "+5:30",
        "+05:3", "-12:00", "+14:00", "+00:00:00.000001", "+05:30:15,5", "+0530:15",
    ]  # fmt: skip
    main_dates = ["2026-01-10", "20260110", "2026-W02-6", "2026W026", "2024-02-29", "9999-12-31", "0001-01-01"]
    product = [d + "T" + t + z for d in main_dates for t in times for z in tzs]
    rng.shuffle(product)
    generated = product[:1100]
    generated += [d + s + "10:00:10Z" for d in dates for s in seps]
    generated += [d + "T10:00:10" + z for d in dates for z in ("Z", "+07:00", "")]
    pool = list("0123456789-:T.,+Z zW") + [SURROGATE, "\u00e9", "\x00", "\n"]
    seeds = hand + generated[:200]
    generated += [mutate(rng, rng.choice(seeds), pool) for _ in range(500)]
    return unique(hand + generated)


def time_entry(text):
    try:
        parsed = datetime.datetime.fromisoformat(text)
    except ValueError:
        why = "invalid"
    else:
        if parsed.tzinfo is None:
            why = "naive"
        else:
            try:
                parsed.astimezone(datetime.UTC)
                why = "ok"
            except Exception as exc:  # the Python ingest does not catch it: it crashes on such a line
                return {"in": text, "out": None, "why": f"crash:{type(exc).__name__}"}
    out = claude_code._utc(text)
    assert (out is not None) == (why == "ok"), text
    return {"in": text, "out": out, "why": why}


# -- labels, words, secrets, Bash first words, WebFetch hosts

ALLOWED = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_.:@/+-"
OTHER = " \t\n\r\x00\x7f!\"#$%&'()*,;<=>?[\\]^`{|}~\u00e9\u00df\u4e2d\u0661\u212a\u0345\u2014\u00a0" + SURROGATE


def label_inputs(rng):
    hand = [
        "", "a", "feature\n", "feature\n\n", "feature\r\n", "\nfeature", "fea\nture", "fea ture", "f\u00e9ature",
        "<synthetic>", "claude-opus-5-5", "2.1.400", "00000000-0000-4000-8000-000000001011", "msg_01ABC", "sdk-cli",
        "mcp__easy-ui__check", "a" * 199, "a" * 200, "a" * 201, "a" * 200 + "\n", "a" * 201 + "\n", "a" * 199 + "\n",
        "\n", "\n\n", "@", "user\n", "a\x00", "a" + SURROGATE, SURROGATE, "\u212a", "x" * 32, "sk-FAKEKEY",
        ALLOWED, ALLOWED + "\n",
    ]  # fmt: skip
    chars = ALLOWED + OTHER
    generated = []
    for _ in range(320):
        n = rng.choice([1, 2, 5, 12, 40, 199, 200, 201, rng.randrange(0, 210)])
        text = "".join(rng.choice(ALLOWED if rng.random() < 0.8 else chars) for _ in range(n))
        if rng.random() < 0.15:
            text += rng.choice(["\n", "\n\n", "\r", " "])
        generated.append(text)
    return unique(hand + generated)


def word_inputs(rng):
    word_chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_.+-"
    hand = [
        "", "ls", "git", "run.sh", "tool-name_v2.1+x", "_x", ".x", "-x", "+x", "a" * 64, "a" * 65, "ls\n", "l s",
        "l\u00e9", "1", "x/y", "x:y", "x@y", "\u212a", "a" + SURROGATE,
    ]  # fmt: skip
    generated = []
    for _ in range(160):
        n = rng.choice([1, 2, 8, 63, 64, 65, rng.randrange(0, 70)])
        generated.append("".join(rng.choice(word_chars if rng.random() < 0.9 else OTHER) for _ in range(n)))
    return unique(hand + generated)


def secret_inputs(rng):
    prefixes = ["sk-a", "sk-", "sk-_", "sk-A1", "ghp_a", "gho_a", "ghu_a", "ghs_a", "ghr_a", "ghx_a", "ghp_", "ghp-a",
                "github_pat_", "github_pa", "xoxb-1234", "xox-1234", "xoxa_1234", "xoxab1234", "xox1234", "xoxb-123",
                "xoxb1234", "xoxb--1234", "xox_-1234", "xoxB_abcd", "AKIA1234", "AKIA123", "ASIAABCD", "akia1234",
                "AKIAabcd", "-----BEGIN", "----BEGIN", "-----begin"]  # fmt: skip
    befores = ["", " ", "a", "_", "1", "-", "/", ".", "\u00e9", "\u4e2d", "\u0661", "\u2014", "\u00a0", SURROGATE, "\n"]
    out = [b + p + a for p in prefixes for b in befores for a in ("", "x")][:420]
    out += ["a" * n for n in (31, 32, 33)] + ["A1" * 16, "A1" * 15 + "A", "x" * 31 + "-" + "x", "x" * 31 + "\u00e9"]
    out += ["00000000-0000-4000-8000-000000001011", "/home/user/projects/app", "abc" * 11]
    chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_- /" + "\u00e9" + SURROGATE
    for _ in range(140):
        base = rng.choice(prefixes) if rng.random() < 0.5 else ""
        text = "".join(rng.choice(chars) for _ in range(rng.randrange(0, 12))) + base
        text += "".join(rng.choice(chars) for _ in range(rng.randrange(0, 40)))
        out.append(text)
    return unique(out)


def bash_inputs(rng):
    hand = [
        "ls -la", "  git status", "/usr/bin/env python3", "./run.sh --x", "", "   ", "\t\n", "ls", "l\u00e9 x",
        "tool-name_v2.1+x arg", "sk-FAKE0000000000000000 run", "A" * 32 + " x", "AKIAFAKE0000 x", "a/b/", "a/b/c",
        "/", "//", "x/", "\u00a0ls", "\x1cls", "\x1fls\x1fx", "\u3000ls", "\u200bls", "\u180els", "\x85ls",
        "ls\u2028x", "ls\u00a0-la", "\x0bls", "\x0cls", "\rls", "ls" + SURROGATE, SURROGATE + " ls", "a" * 64,
        "a" * 65, "-rf /", "_x", "1abc", ".x", "npm\u3000run", "git\x1d", "sk-ant-api03-x run", "ghp_x run",
    ]  # fmt: skip
    spaces = [" ", "\t", "\n", "\u00a0", "\x1c", "\u2003", "\u3000", "\u200b", "\x85", "\u202f"]
    words = ["ls", "git", "/bin/sh", "./x.sh", "a/b", "AKIA1234", "sk-x", "x" * 32, "\u00e9", "npm", "-x", "a+b"]
    generated = []
    for _ in range(140):
        generated.append(
            rng.choice(["", rng.choice(spaces)]) + rng.choice(words) + rng.choice(spaces) + rng.choice(words)
        )
    return unique(hand + generated)


def host_inputs(rng):
    hand = [
        "https://user:pass@Example.COM:8443/path?q=1#f", "http://localhost:3000/", "http://127.0.0.1:8080/x",
        "ftp://example.com/", "not a url", "https://example.com./", "https://docs.example.com/a",
        "HTTP://Example.com", "https://example", "https://-a.com/", "https://a-.com/", "https://a--b.com/",
        "https://" + "a" * 63 + ".com", "https://" + "a" * 64 + ".com", "https://" + "a" * 65 + ".com",
        "https://a.b.c.d.e", "https://1.2", "https://a..b", "https://.a.com", "https://a.com..",
        "https://[::1]/", "https://[v1.ab]/", "https://[V1.ab]/", "https://[v1.ab\n]/", "https://[x]@v1.ab/",
        "https://[x]@example.com/", "https://[::1]@example.com/", "https://a[b@example.com/",
        "https://exa]mple.com/", "https://x[::1]/", "https://[::1]x/", "https://[::1]:80/",
        "https://\u212aexample.com/", "https://\u00fc@example.com/", "https://\ufe6b@example.com/",
        "https://\u2100.example.com/", "https://ex\u00e4mple.com/", "https://\uff45xample.com/",
        "https://" + SURROGATE + "@example.com/", "https://example.com" + SURROGATE + "/",
        "  https://example.com", "\x00https://example.com", "ht\ttps://exa\nmple.com", "https:example.com",
        "https:/example.com", "https:///example.com", "https://example.com:abc/", "https://example.com:99999/",
        "https://@example.com", "https://a@b@example.com", "https://example.com\\x", "https://example.com?x",
        "https://example.com#x", "1https://example.com", "+https://example.com", "https+x://example.com",
        "https://EXAMPLE.com.", "https://example.com...", "https://exa_mple.com", "https://%65xample.com",
        "https://example.com%2f", "https://fe80::1%eth0", "https://[fe80::1%25eth0]/", "javascript:alert(1)",
        "", "https://", "http://a.b", "https://example.com:443", "https://xn--mnchen-3ya.de/",
    ]  # fmt: skip
    hosts = ["example.com", "Example.COM", "127.0.0.1", "localhost", "a.b", "a-b.c-d", "x" * 64 + ".io", "a.com."]
    schemes = ["http://", "https://", "HTTPS://", "ftp://", "", "//", "http:/"]
    extras = ["", "/", ":80", ":80/x", "?q", "#f", "/a b", "@x.com"]
    generated = [s + rng.choice(["", "u@", "u:p@"]) + h + e for s in schemes for h in hosts for e in extras]
    rng.shuffle(generated)
    return unique(hand + generated[:120])


def label_corpus(rng):
    def py_split_space(c):
        return len(("a" + chr(c) + "b").split()) == 2

    entries = {
        "label": [{"in": s, "out": claude_code._label(s)} for s in label_inputs(rng)],
        "word": [{"in": s, "out": bool(claude_code._WORD.fullmatch(s))} for s in word_inputs(rng)],
        "secret": [{"in": s, "out": SECRET.search(s) is not None} for s in secret_inputs(rng)],
        "bash": [{"in": s, "out": claude_code._bash_word(s)} for s in bash_inputs(rng)],
        "host": [{"in": s, "out": claude_code._host(s)} for s in host_inputs(rng)],
    }
    code_points = [c for c in range(0x110000) if not 0xD800 <= c < 0xE000]
    tables = {
        "split_whitespace": [c for c in code_points if py_split_space(c)],
        "nfkc_netloc_forbidden": [
            c for c in code_points if c > 0x7F and any(x in unicodedata.normalize("NFKC", chr(c)) for x in "/?#@:")
        ],
        "lower_to_ascii": [c for c in code_points if c > 0x7F and chr(c).lower().isascii()],
    }
    return entries, tables


def build():
    rng = random.Random(SEED)
    meta = {
        "generator": "rust/parity/gen_ingest_corpus.py",
        "python": platform.python_version(),
        "seed": SEED,
        "unicodedata": unicodedata.unidata_version,
    }
    times = [time_entry(t) for t in time_inputs(rng)]
    entries, tables = label_corpus(rng)
    counts = {k: len(v) for k, v in entries.items()}
    return {
        "corpus-time.json": {**meta, "entries": times, "count": len(times)},
        "corpus-label.json": {**meta, **entries, "counts": counts, "tables": tables},
    }


def render(obj) -> bytes:
    return (json.dumps(obj, indent=1, ensure_ascii=True, sort_keys=True) + "\n").encode()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="compare with the committed files, write nothing")
    args = parser.parse_args()
    files = {name: render(obj) for name, obj in build().items()}
    if args.check:
        different = [n for n, data in files.items() if not (GOLDEN / n).is_file() or (GOLDEN / n).read_bytes() != data]
        print(f"--check: {len(files)} files, {len(different)} different" + "".join(f"\n  {n}" for n in different))
        return 1 if different else 0
    for name, data in files.items():
        (GOLDEN / name).write_bytes(data)
        print(f"wrote {name} ({len(data)} bytes)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
