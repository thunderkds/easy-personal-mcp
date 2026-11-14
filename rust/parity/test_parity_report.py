#!/usr/bin/env python3
"""Completeness test of the T061 parity report (AC1, AC4, AC5, AC6): run it with the parity venv.

    ./parity/.venv/bin/python parity/test_parity_report.py        # from rust/; exit 0 when every test passes

It reads the Python app's own surfaces (argparse tree, route table, MCP tool and prompt lists: `run_parity.inventory()`) and
fails when `docs/PARITY_T061.md` lacks a row for one of them. It does not run the apps (that is `run_parity.py`).
"""

import re
import sys
import traceback
from pathlib import Path

sys.dont_write_bytecode = True
PARITY = Path(__file__).resolve().parent
sys.path.insert(0, str(PARITY))
import run_parity as rp  # noqa: E402


def report_text() -> str:
    assert rp.REPORT.exists(), f"{rp.REPORT} does not exist: run parity/run_parity.py first"
    return rp.REPORT.read_text()


def test_every_python_surface_has_a_row():
    rows = rp.parse_report(report_text())
    keys = [r["key"] for r in rows]
    missing = rp.missing_surfaces(rp.inventory(), keys)
    assert not missing, f"surfaces of the Python app that the table lacks: {missing}"
    dup = sorted({k for k in keys if keys.count(k) > 1})
    assert not dup, f"surfaces listed twice: {dup}"


def test_a_new_python_surface_would_fail_the_check():
    keys = [r["key"] for r in rp.parse_report("| `cli: ping` | 1 | equal |  |\n")]
    assert rp.missing_surfaces(["cli: ping", "cli: brand-new", "route: GET /api/new", "mcp tool: new"], keys) == [
        "cli: brand-new", "mcp tool: new", "route: GET /api/new"]
    assert rp.missing_surfaces([], keys) == []


def test_the_inventory_is_read_from_python_and_is_not_small():
    inv = rp.inventory()
    kinds = {k.split(":")[0] for k in inv}
    assert kinds == {"cli", "route", "mcp tool", "mcp prompt"}, kinds
    assert len(inv) >= 80, len(inv)
    for known in ("cli: usage", "cli: connector adopt-legacy", "route: POST /api/sync", "mcp tool: save_report", "mcp prompt: investigate"):
        assert known in inv, known


def test_every_row_has_a_valid_result_and_the_counts_add_up():
    text = report_text()
    rows = rp.parse_report(text)
    assert rows, "no table rows"
    for row in rows:
        assert rp.CLASS.match(row["result"]), f"{row['key']}: bad result {row['result']!r}"
        assert row["probes"] >= 1 or row["result"].startswith("not run"), f"{row['key']}: no probe"
    found = rp.COUNTS.search(text)
    assert found, "no `Counts:` line at the top"
    want = [int(x) for x in found.groups()]
    got = [sum(r["result"] == "equal" for r in rows), sum(r["result"].startswith("differs: README") for r in rows),
           sum(r["result"] == "differs: new" for r in rows), sum(r["result"].startswith("not run") for r in rows)]
    assert want == got, f"counts line {want}, table {got}"


def test_each_readme_item_cited_exists_and_says_it():
    text = report_text()
    cited = {(r, n) for row in rp.parse_report(text) for r, n in rp.readme_refs(row["result"])}
    for ref, number in sorted(cited):
        words = rp.EXPLAIN_WORDS.get((ref, number))
        assert words, f"{ref} item {number} is cited but has no stated words in run_parity.EXPLAIN_WORDS"
        assert rp.readme_ok(ref, number, words), f"README `Decided for you ({ref}...` item {number} is missing or lacks {words}"


def test_each_new_finding_has_a_reproduction_and_a_proposed_task():
    text = report_text()
    for row in rp.parse_report(text):
        if row["result"] != "differs: new":
            continue
        section = re.search(r"(?ms)^### Finding: `" + re.escape(row["key"]) + r"`.*?(?=^### |^## |\Z)", text)
        assert section, f"{row['key']}: no finding section"
        body = section.group(0)
        assert "Reproduce:" in body and "Python:" in body and "Rust:" in body and "Proposed task:" in body, f"{row['key']}: incomplete finding"


def test_the_report_holds_no_canary_and_the_scanner_sees_one():
    c = rp.canaries()
    planted = next(iter(c.values.values()))
    assert rp.scan("x " + planted + " y"), "the scanner did not see a planted canary"
    assert rp.scan(planted.encode().hex()), "the scanner did not see a hex canary"
    hits = rp.scan(report_text())
    assert not hits, f"the report holds a canary: {hits}"
    assert "/tmp/" not in report_text() and "parity-world" not in report_text(), "the report holds a temp path"


def test_normalising_does_not_hide_a_difference():
    a = {"code": 0, "stdout": '{"as_of": "2026-01-01", "n": 1}', "stderr": ""}
    b = {"code": 0, "stdout": '{"as_of": "2030-05-05", "n": 1}', "stderr": ""}
    norm = lambda o: {k: rp.scrub_text(v, None) for k, v in o.items()}  # noqa: E731
    assert rp.compare(norm(a), norm(b)), "N3 must make the clock's value equal"
    c = {**b, "stdout": '{"as_of": "2030-05-05", "n": 2}'}
    assert not rp.compare(norm(a), norm(c)), "a changed number must stay a difference"
    assert not rp.compare({"code": 0, "stdout": "", "stderr": ""}, {"code": 1, "stdout": "", "stderr": ""}), "an exit code must count"
    assert rp.scrub_text("easy-personal-rs: x\n", None) == "easy-personal: x\n"
    assert rp.scrub_text("see easy-personal-rs: x\n", None) == "see easy-personal-rs: x\n", "N2 is the line start only"


def test_a_readme_item_is_found_and_a_wrong_number_is_not():
    assert rp.readme_item("T058", 1) and "missing store" in rp.readme_item("T058", 1).lower()
    assert rp.readme_item("T058", 999) is None
    assert rp.readme_item("T999", 1) is None
    assert not rp.readme_ok("T058", 1, ("a phrase that is not there",))


def zone(code, stdout, stderr):
    return {"steps": [{"code": code, "stdout": stdout, "stderr": stderr}], "state": {"store_changed": {}, "keychain": "none", "claude_calls": [], "claude_state": {}}}


def test_a_readme_rule_explains_only_what_it_describes():
    probe = {"explain": ["T058#8"]}
    argparse_error = "usage: easy-personal usage [-h]\neasy-personal usage: error: argument --by: invalid choice: 'x'\n"
    clap_error = "error: invalid value 'x' for '--by <BY>'\n\nFor more information, try '--help'.\n"
    assert rp.classify_zone(probe, zone(2, "", argparse_error), zone(2, "", clap_error)) == ("readme", ["T058#8"])
    # the same stderr difference, but the exit codes differ: the item says "exit code 2 in both", so this is a finding
    assert rp.classify_zone(probe, zone(2, "", argparse_error), zone(1, "", clap_error)) == ("new", [])
    # stdout differs as well: not described
    assert rp.classify_zone(probe, zone(2, "", argparse_error), zone(2, "extra", clap_error)) == ("new", [])
    # no rule declared: any difference is a finding
    assert rp.classify_zone({"explain": []}, zone(2, "", argparse_error), zone(2, "", clap_error)) == ("new", [])
    assert rp.classify_zone({"explain": []}, zone(0, "a", ""), zone(0, "a", "")) == ("equal", [])


def test_a_missing_store_is_explained_only_when_python_made_the_store():
    probe = {"explain": ["T058#1"]}
    refused = zone(1, "", rp.NO_STORE_TEXT)
    made = zone(0, "{}\n", "")
    made["state"]["store_changed"] = {"workspaces": [[1, "orphan"]]}
    assert rp.classify_zone(probe, made, refused) == ("readme", ["T058#1"])
    assert rp.classify_zone(probe, zone(0, "{}\n", ""), refused) == ("new", []), "Python did not make a store: not what the item says"
    assert rp.classify_zone(probe, made, zone(1, "", "easy-personal: some other refusal\n")) == ("new", [])


def main() -> int:
    failed = 0
    for name, fn in sorted((n, f) for n, f in globals().items() if n.startswith("test_") and callable(f)):
        try:
            fn()
            print(f"ok   {name}")
        except Exception:  # noqa: BLE001 - shown, then a failing exit code
            failed += 1
            print(f"FAIL {name}")
            traceback.print_exc(limit=2)
    print(f"{'FAILED' if failed else 'passed'}: {failed} failing")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
