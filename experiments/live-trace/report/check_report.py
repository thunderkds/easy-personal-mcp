"""Report integrity check (HUN-170). Exit 1 and print the problems when:
  * a RESULT.md lacks a required section (Tried / Worked / Failed / Numbers / Edge cases / Verdict),
  * REPORT.md lacks a template section or the Comparison table lacks a row for a source,
  * report/e5.md lacks the 'docs only, not tested' marker,
  * a planted test secret or fake prompt sentence appears in any stored file outside tests/.
Usage: python check_report.py            (checks the real files under experiments/live-trace/)
"""
import re
import sys
from pathlib import Path

LT = Path(__file__).resolve().parents[1]
RESULT_FILES = ("harness/RESULT.md", "e1/RESULT.md", "e2/RESULT.md", "e3/RESULT.md", "e4/RESULT.md")
RESULT_SECTIONS = ("Tried", "Worked", "Failed", "Numbers", "Edge cases", "Verdict")
REPORT_SECTIONS = ("Recommendation", "Comparison", "Account usage findings",
                   "Risks and open questions", "v2 ideas")
COMPARISON_COLUMNS = ("Source", "Median latency", "Completeness", "Setup effort", "Overhead", "Verdict")
REPORT_SOURCES = ("E1", "E2", "E3", "E4", "E5")
E5_MARKER = "docs only, not tested"
# Built from pieces so this file never holds the literal strings it searches for.
PLANTED = ("sk-ant-" + "FAKE-SECRET", "quarterly tax " + "plan", "quarterly numbers for the " + "Zebra")
TEXT_SUFFIXES = {".md", ".json", ".jsonl", ".txt", ".py", ".toml", ".yaml", ".yml", ".sh"}


def headings(text):
    return [ln[3:].strip() for ln in text.splitlines() if ln.startswith("## ")]


def check_result(path):
    p = Path(path)
    if not p.is_file():
        return [f"{p}: file missing"]
    found = headings(p.read_text())
    return [f"{p}: missing section '## {s}'" for s in RESULT_SECTIONS
            if not any(h == s or h.startswith(s) for h in found)]


def comparison_rows(text):
    """(header cells, [row first cells]) of the table under '## Comparison'."""
    m = re.search(r"^## Comparison\s*$(.*?)(?=^## |\Z)", text, re.S | re.M)
    if not m:
        return [], []
    tbl = [ln for ln in m.group(1).splitlines() if ln.strip().startswith("|")]
    cells = [[c.strip() for c in ln.strip().strip("|").split("|")] for ln in tbl]
    if not cells:
        return [], []
    return cells[0], [c[0] for c in cells[2:] if c]


def check_report(path):
    p = Path(path)
    if not p.is_file():
        return [f"{p}: file missing"]
    text = p.read_text()
    probs = []
    if not text.startswith("# Claude Live Trace"):
        probs.append(f"{p}: title must start with '# Claude Live Trace'")
    found = headings(text)
    probs += [f"{p}: missing section '## {s}'" for s in REPORT_SECTIONS if s not in found]
    header, firsts = comparison_rows(text)
    probs += [f"{p}: Comparison table lacks column '{c}'" for c in COMPARISON_COLUMNS if c not in header]
    probs += [f"{p}: Comparison table lacks a row for {s}" for s in REPORT_SOURCES
              if not any(f.replace("*", "").strip().startswith(s) for f in firsts)]
    return probs


def check_e5(path):
    p = Path(path)
    if not p.is_file():
        return [f"{p}: file missing"]
    return [] if E5_MARKER in p.read_text().lower() else [f"{p}: missing marker '{E5_MARKER}'"]


def check_secrets(root=LT):
    probs = []
    for f in sorted(Path(root).rglob("*")):
        rel = f.relative_to(root)
        if not f.is_file() or "tests" in rel.parts or "__pycache__" in rel.parts:
            continue
        if f.suffix not in TEXT_SUFFIXES:
            continue
        text = f.read_text(errors="replace")
        probs += [f"{rel}: planted test string found" for s in PLANTED if s in text][:1]
    return probs


def check_all(root=LT):
    root = Path(root)
    probs = []
    for rel in RESULT_FILES:
        probs += check_result(root / rel)
    probs += check_report(root / "REPORT.md")
    probs += check_e5(root / "report" / "e5.md")
    probs += check_secrets(root)
    return probs


def main():
    probs = check_all()
    for p in probs:
        print("FAIL:", p)
    print("report check:", "FAILED" if probs else "ok", f"({len(probs)} problem(s))")
    return 1 if probs else 0


if __name__ == "__main__":
    sys.exit(main())
