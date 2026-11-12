#!/usr/bin/env python3
"""Oracle for the Rust `secret::word` (T052): Python's own `\\w` answer for every Unicode code point.

    rust/parity/.venv/bin/python rust/parity/gen_word_table.py            # writes rust/parity/golden-save-report/word_table.json
    rust/parity/.venv/bin/python rust/parity/gen_word_table.py --check    # regenerates in memory; exit 0 equal, 1 different

`core/secretlike.py`'s `SECRET` uses `\\b`, which looks at `\\w`: for a `str` pattern, `str.isalnum()` or `_`. Rust's
`char::is_alphanumeric` is not the same set: it also counts Other_Alphabetic characters that are not letters or digits
(circled and squared letters U+24B6-U+24E9, U+1F130-U+1F149, U+1F150-U+1F169, U+1F170-U+1F189, and combining marks such as
U+0345 and U+093F), so a secret behind one of them was judged differently. The file holds Python's answer as sorted,
inclusive `[low, high]` ranges of the code points that are word characters (surrogates left out: no Rust `char` is one);
`secret.rs` carries the few ranges where `is_alphanumeric` differs and its test compares every code point with this file.
The ranges depend on the Unicode version of the Python that makes the file (`unicode` says which).

Python is only read; nothing outside `rust/parity/` is written.
"""

import argparse
import json
import re
import sys
import unicodedata
from pathlib import Path

PARITY = Path(__file__).resolve().parent
OUT = PARITY / "golden-save-report" / "word_table.json"
WORD = re.compile(r"\w")


def table() -> dict:
    ranges: list[list[int]] = []
    for code in range(0x110000):
        if 0xD800 <= code <= 0xDFFF:
            continue
        if WORD.fullmatch(chr(code)) is None:
            continue
        if ranges and ranges[-1][1] == code - 1:
            ranges[-1][1] = code
        else:
            ranges.append([code, code])
    return {
        "generator": "rust/parity/gen_word_table.py",
        "python": ".".join(map(str, sys.version_info[:3])),
        "unicode": unicodedata.unidata_version,
        "word_ranges": ranges,
    }


def text(data: dict) -> str:
    return json.dumps(data, sort_keys=True, separators=(",", ":")).replace("],[", "],\n[") + "\n"


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--check", action="store_true")
    made = text(table())
    if parser.parse_args().check:
        same = OUT.is_file() and OUT.read_text() == made
        print("word_table.json: " + ("equal" if same else "DIFFERENT"))
        sys.exit(0 if same else 1)
    OUT.write_text(made)
    print(f"{len(table()['word_ranges'])} ranges -> {OUT}")
