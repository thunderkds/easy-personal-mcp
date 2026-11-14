#!/usr/bin/env python3
"""Completeness test of the T061 parity report (AC1, AC4, AC5, AC6): run it with the parity venv.

    ./parity/.venv/bin/python parity/test_parity_report.py        # from rust/; exit 0 when every test passes

It reads the Python app's own surfaces (argparse tree, route table, MCP tool and prompt lists: `run_parity.inventory()`) and
fails when `docs/PARITY_T061.md` lacks a row for one of them. It does not run the apps (that is `run_parity.py`).
"""

import json
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
    got = [sum(r["result"] == "equal" for r in rows), sum(r["result"] == "equal except JSON key order" for r in rows),
           sum(r["result"].startswith("differs: README") for r in rows), sum(r["result"] == "differs: new" for r in rows),
           sum(r["result"].startswith("not run") for r in rows)]
    assert want == got, f"counts line {want}, table {got}"
    for row in rows:
        if row["result"] == "equal except JSON key order":
            assert "key order" in row["note"], f"{row['key']}: the note must say how many bodies differ in key order"
    assert "rust/parity/README.md" in text and "Key order: which rule wins (T007)" in text and "T059 item 6" in text, "the key-order rule is not named"
    assert "MCP result key order is not tracked" in text, "the report must say that MCP key order is not tracked"
    assert "## Known limits" in text, "no Known limits section"


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


def test_the_runner_does_not_depend_on_an_inherited_ignored_sigint():
    """A job started in the background of a non-interactive shell has SIGINT ignored; `exec` keeps that, so a server child would not die by
    SIGINT. The runner must give its children a default SIGINT: Python's `mcp --transport http` ends by the signal (exit -2), and the
    probe shows it as `step 4 /code`."""
    import os
    import signal
    import subprocess

    env = {"PATH": "/usr/bin:/bin", "HOME": os.environ.get("HOME", "/"), "PYTHONDONTWRITEBYTECODE": "1"}
    done = subprocess.run([sys.executable, str(PARITY / "run_parity.py"), "--only", "cli: mcp / http-handshake-and-ping"], capture_output=True,
                          text=True, env=env, cwd=str(PARITY.parent), check=False, timeout=300,
                          preexec_fn=lambda: signal.signal(signal.SIGINT, signal.SIG_IGN))
    # T067: the probe is `differs: README T051 item 8` only when Python's exit code is -2; a child that ignored SIGINT would show `step 4 /code`
    assert done.stdout.startswith("README cli: mcp / http-handshake-and-ping") and "T051#8" in done.stdout.splitlines()[0], (
        f"the HTTP MCP server did not end by SIGINT (exit -2):\n{done.stdout[-600:]}\n{done.stderr[-300:]}")


def test_the_usage_error_texts_are_explained_by_item_14_and_only_when_exact():
    """T058 item 8 says Rust prints clap's text; `--by week` and `--since banana` are the core's one line (T067 added item 14 for them)."""
    argparse_by = "usage: easy-personal usage [-h]\neasy-personal usage: error: argument --by: invalid choice: 'week' (choose from day, model, project, tool)\n"
    core_by = "easy-personal: by must be one of day, model, project, tool\n"
    assert rp.classify_zone({"explain": ["T058#8"]}, zone(2, "", argparse_by), zone(2, "", core_by)) == ("new", []), "item 8 is clap's text, not this one"
    assert rp.classify_zone({"explain": ["T058#14"]}, zone(2, "", argparse_by), zone(2, "", core_by)) == ("readme", ["T058#14"])
    argparse_date = "usage: easy-personal usage [-h]\neasy-personal usage: error: argument --since: invalid iso_datetime value: 'banana'\n"
    core_date = "easy-personal: Invalid isoformat string: 'banana'\n"
    assert rp.classify_zone({"explain": ["T058#14"]}, zone(2, "", argparse_date), zone(2, "", core_date)) == ("readme", ["T058#14"])
    # mutations: one other byte differs, so the difference is not the one the item names
    assert rp.classify_zone({"explain": ["T058#14"]}, zone(2, "", argparse_date), zone(2, "", "easy-personal: Invalid isoformat string: 'bananas'\n")) == ("new", [])
    assert rp.classify_zone({"explain": ["T058#14"]}, zone(2, "", argparse_by), zone(2, "", core_by.replace("project", "projects"))) == ("new", [])
    assert rp.classify_zone({"explain": ["T058#14"]}, zone(2, "", argparse_by), zone(1, "", core_by)) == ("new", []), "the exit code must be 2 in both"
    assert rp.classify_zone({"explain": ["T058#14"]}, zone(2, "", argparse_by), zone(2, "x", core_by)) == ("new", []), "stdout must be empty"
    assert "T058#7" not in rp.RULES, "item 7 does not describe a different text: no rule may stand for it"
    rp.define_all()
    expects = {(p["key"], p["id"]): p["explain"] for p in rp.PROBES}
    assert expects[("cli: usage", "bad-by")] == ["T058#14"] and expects[("cli: usage", "bad-date")] == ["T058#14"]
    clap = "error: invalid value 'colour' for '--by <BY>'\n  [possible values: account, provider]\n\nFor more information, try '--help'.\n"
    assert rp.classify_zone({"explain": ["T058#8"]}, zone(2, "", "usage: easy-personal account list [-h]\n...error: invalid choice: 'colour'\n"),
                            zone(2, "", clap)) == ("readme", ["T058#8"]), "clap's own text is what item 8 describes"


def test_the_blind_check_fails_a_probe_that_errors_on_both_sides():
    ok = {"py": {"UTC": {"steps": [{"code": 0, "stdout": "x", "stderr": ""}]}}, "rs": {"UTC": {"steps": [{"code": 0, "stdout": "x", "stderr": ""}]}}}
    assert rp.blind_failure(ok) is None
    both = lambda step: {"py": {"UTC": {"steps": [step]}}, "rs": {"UTC": {"steps": [step]}}}  # noqa: E731
    for step in ({"code": "timeout", "stdout": "", "stderr": ""}, {"code": "did not stop"}, {"status": "error: TimeoutError", "type": None, "body": ""},
                 {"method": "m", "answer": {"error": "no answer within 60 s"}}, {"method": "m", "answer": {"error": "the server closed its output"}},
                 {"method": "m", "answer": {"error": "not JSON", "text": "x"}}, {"code": 0, "stdout": "", "stderr": "", "api_status": None},
                 {"step": "initialize", "status": 500, "form": "error"}, {"step": "ping-tool", "status": 404, "form": "error"}):
        assert rp.blind_failure(both(step)), f"not seen as a failed run: {step}"
    # an error the app is meant to give is not blind
    for step in ({"method": "m", "answer": {"error": {"code": -32601, "message": "x"}}}, {"status": 403, "type": "application/json", "body": {}},
                 {"step": "bad-json", "status": 400, "form": "error"}, {"code": 2, "stdout": "", "stderr": "usage: x"},
                 {"code": 0, "stdout": "", "stderr": "", "api_status": 200}):
        assert rp.blind_failure(both(step)) is None, f"wrongly seen as a failed run: {step}"


def test_a_bare_saved_utc_column_is_masked_and_nothing_else_in_the_row():
    """The stored `saved_utc` of a new report is the clock of the second it was saved in: two apps run a moment apart."""
    row = lambda stamp, version, summary: [3, stamp, version, None, None, summary, "[]", "{}"]  # noqa: E731
    state = lambda r: {"store_changed": {"investigate_reports": [r]}, "keychain": "none", "claude_calls": [], "claude_state": {}}
    a, b = rp.scrubbed_state(state(row("2026-11-14T07:32:27Z", "0.4.0", "s")), None), rp.scrubbed_state(state(row("2026-11-14T07:32:36Z", "0.4.0", "s")), None)
    assert rp.compare(a, b), "a gap of 9 seconds in saved_utc must not be a difference"
    c = rp.scrubbed_state(state(row("2026-11-14T07:32:36Z", "0.4.0", "another summary")), None)
    assert not rp.compare(a, c), "the summary column is still compared"
    d = rp.scrubbed_state(state(row("2026-11-14T07:32:36Z", "0.9.9", "s")), None)
    assert not rp.compare(a, d), "the version column is compared until T047#6 stands for it"


def route_zone(status, ctype, body):
    return {"steps": [{"status": status, "type": ctype, "body": body}], "state": zone(0, "", "")["state"]}


def test_the_static_rule_explains_only_the_fixed_text_of_a_miss_and_a_refused_method():
    probe = {"explain": ["Static#1"]}
    assert rp.classify_zone(probe, route_zone(404, "text/plain; charset=utf-8", "Not Found"), route_zone(404, None, "")) == ("readme", ["Static#1"])
    assert rp.classify_zone(probe, route_zone(405, "text/plain; charset=utf-8", "Method Not Allowed"), route_zone(405, None, "")) == ("readme", ["Static#1"])
    assert rp.classify_zone(probe, route_zone(404, "text/plain; charset=utf-8", "Not Found"), route_zone(200, None, "")) == ("new", [])
    assert rp.classify_zone(probe, route_zone(404, "text/plain; charset=utf-8", "Gone"), route_zone(404, None, "")) == ("new", [])
    assert rp.classify_zone(probe, route_zone(404, "text/plain; charset=utf-8", "Not Found"), route_zone(404, "application/json", "")) == ("new", [])


def test_the_ingest_rule_explains_only_the_extra_skipped_key():
    probe = {"explain": ["Ingest#1"]}
    py = zone(0, json.dumps({"files": 4, "new_events": 18}, indent=2, sort_keys=True) + "\n", "")
    same = zone(0, json.dumps({"files": 4, "new_events": 18, "skipped": 0}, indent=2, sort_keys=True) + "\n", "")
    other = zone(0, json.dumps({"files": 4, "new_events": 17, "skipped": 0}, indent=2, sort_keys=True) + "\n", "")
    assert rp.classify_zone(probe, py, same) == ("readme", ["Ingest#1"])
    assert rp.classify_zone(probe, py, other) == ("new", []), "a different count is not what the item says"


def test_the_doctor_rule_explains_only_the_name_of_the_keychain_backend():
    probe = {"explain": ["T060#5"]}
    py = zone(0, "ok keychain: FileKeyring\nok claude: on PATH\n", "")
    assert rp.classify_zone(probe, py, zone(0, "ok keychain: FileKeychain\nok claude: on PATH\n", "")) == ("readme", ["T060#5"])
    assert rp.classify_zone(probe, py, zone(0, "ok keychain: FileKeychain\nFAIL claude: not on PATH\n", "")) == ("new", [])
    assert rp.classify_zone(probe, py, zone(1, "ok keychain: FileKeychain\nok claude: on PATH\n", "")) == ("new", [])


def test_the_save_report_rules_explain_only_the_version_and_the_argument_error_text():
    row = lambda version, summary: [3, "<NOW>", version, None, None, summary]  # noqa: E731
    state = lambda r: {"store_changed": {"investigate_reports": [r]}, "keychain": "none", "claude_calls": [], "claude_state": {}}
    z = lambda r: {"steps": [], "state": state(r)}  # noqa: E731
    probe = {"explain": ["T047#6"]}
    assert rp.classify_zone(probe, z(row("0.4.0", "s")), z(row("0.2.0", "s"))) == ("readme", ["T047#6"])
    assert rp.classify_zone(probe, z(row("0.4.0", "s")), z(row("0.2.0", "t"))) == ("new", [])
    call = lambda text: {"method": "tools/call", "answer": {"result": {"content": [{"type": "text", "text": text}], "isError": True}}}  # noqa: E731
    mz = lambda text: {"steps": [call(text)], "state": state(row("0.4.0", "s"))}  # noqa: E731
    probe = {"explain": ["T047#1"]}
    py_text = "Error executing tool save_report: 1 validation error for save_reportArguments\nsummary\n  Field required [type=missing]"
    assert rp.classify_zone(probe, mz(py_text), mz("Error executing tool save_report: argument `summary` is required and must be a string")) == ("readme", ["T047#1"])
    assert rp.classify_zone(probe, mz(py_text), mz("something else entirely")) == ("new", [])


def test_the_leak_scan_covers_the_captured_outputs():
    planted = next(iter(rp.canaries().values.values()))
    assert rp.check_leaks("a clean report", ["clean output", "another"]) == 2
    try:
        rp.check_leaks("a clean report", ["clean output", f"stdout holds {planted} here"])
    except SystemExit as stop:
        assert "canary" in str(stop)
    else:
        raise AssertionError("a canary in a captured output was not refused")


def test_the_inventory_leaves_no_temp_folder_behind():
    import glob
    import tempfile

    before = set(glob.glob(str(Path(tempfile.gettempdir()) / "tmp*")))
    rp.inventory()
    after = set(glob.glob(str(Path(tempfile.gettempdir()) / "tmp*")))
    assert after <= before, f"left behind: {sorted(after - before)}"


def test_python_is_always_told_which_static_folder_to_serve():
    """Python's `create_app` serves the packaged dashboard bundle (`src/easy_personal/dashboard/static/`, a gitignored build output) when it exists.
    A probe must not depend on whether the machine has built it: the Python child always gets an explicit folder, a missing one for "no page"."""
    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / "static").mkdir()
        assert rp.static_arg(root, True) == str(root / "static")
        no_page = rp.static_arg(root, False)
        assert no_page and not Path(no_page).exists(), "a probe without a page must point at a folder that does not exist"
    assert "sys.argv[1]" in rp.PY_SERVE and "if len(sys.argv) > 1" not in rp.PY_SERVE, "the folder is not optional"


def test_a_built_bundle_on_this_machine_does_not_change_the_guard_probe():
    import subprocess

    bundle = rp.REPO / "src" / "easy_personal" / "dashboard" / "static"
    made = not bundle.exists()
    if made:
        bundle.mkdir(parents=True)
        (bundle / "index.html").write_text("<!doctype html><title>throwaway</title>\n")
    try:
        env = {"PATH": "/usr/bin:/bin", "HOME": "/", "PYTHONDONTWRITEBYTECODE": "1"}
        done = subprocess.run([sys.executable, str(PARITY / "run_parity.py"), "--only", "route: GET /api/workspaces / guard"], capture_output=True,
                              text=True, env=env, cwd=str(PARITY.parent), check=False, timeout=300)
    finally:
        if made:
            import shutil

            shutil.rmtree(bundle.parent, ignore_errors=True)  # `dashboard/` held only the throwaway bundle (the folder is a gitignored build output)
    first = done.stdout.splitlines()[0] if done.stdout else done.stderr[-300:]
    assert first.startswith("README") and "Static#1" in first, f"the guard probe depends on the bundle: {first}\n{done.stdout[:600]}"


def test_the_guard_is_also_probed_with_the_page_mounted_as_in_a_real_deployment():
    rp.define_all()
    probes = [p for p in rp.PROBES if p["key"] == "route: GET /api/workspaces" and p["id"] == "guard-with-page"]
    assert probes and probes[0]["spec"]["static"] is True, "no probe of the guard with the page served"
    assert "T048#15" in probes[0]["explain"], "Python answers 404 for a GET of a write-only route when the page is mounted; Rust 405 (README T048 item 15)"


def test_version_rule_explains_only_the_version_text():
    probe = {"explain": ["T058#13"]}
    assert rp.classify_zone(probe, zone(0, "0.4.0\n", ""), zone(0, "easy-personal-rs 0.2.0\n", "")) == ("readme", ["T058#13"])
    assert rp.classify_zone(probe, zone(0, "0.4.0\n", ""), zone(0, "easy-personal-rs 0.2.0 extra\n", "")) == ("new", [])
    assert rp.classify_zone(probe, zone(0, "0.4.0\n", ""), zone(0, "other-name 0.2.0\n", "")) == ("new", [])
    assert rp.classify_zone(probe, zone(0, "0.4.0\n", ""), zone(1, "easy-personal-rs 0.2.0\n", "")) == ("new", []), "the exit code is still compared"
    assert rp.classify_zone(probe, zone(0, "0.4.0\n", ""), zone(0, "easy-personal-rs 0.2.0\n", "warning\n")) == ("new", []), "stderr is still compared"


def http_zone(code, stdout):
    return {"steps": [{"step": "ping-tool", "status": 200}, {"step": "exit", "code": code, "stdout": stdout}], "state": zone(0, "", "")["state"]}


def test_ctrl_c_rule_explains_only_the_signal_exit_and_the_access_log():
    probe = {"explain": ["T051#8"]}
    log = 'INFO:     127.0.0.1:<PORT> - "POST /mcp HTTP/1.1" 200 OK\nINFO:     127.0.0.1:<PORT> - "POST /mcp HTTP/1.1" 202 Accepted\n'
    assert rp.classify_zone(probe, http_zone(-2, log), http_zone(0, "")) == ("readme", ["T051#8"])
    assert rp.classify_zone(probe, http_zone(-2, log + "Traceback (most recent call last):\n"), http_zone(0, "")) == ("new", []), "other stdout text is not an access log"
    assert rp.classify_zone(probe, http_zone(-2, log), http_zone(0, "one line\n")) == ("new", []), "Rust's stdout must be empty"
    assert rp.classify_zone(probe, http_zone(-2, log), http_zone(1, "")) == ("new", []), "Rust's exit code must be 0"
    assert rp.classify_zone(probe, http_zone(-9, log), http_zone(0, "")) == ("new", []), "Python must end by SIGINT (-2)"
    assert rp.classify_zone(probe, http_zone("did not stop", log), http_zone(0, "")) == ("new", [])


def mcp_zone(steps, stderr="", exit_code=0):
    return {"steps": steps, "state": zone(0, "", "")["state"], "exit": exit_code, "stderr": stderr}


def answer(method, **kw):
    return {"method": method, "params": None, "answer": {"jsonrpc": "2.0", "id": 1, **kw}}


def test_prompt_rule_explains_only_the_code_and_the_traceback():
    probe = {"explain": ["T047#7"]}
    trace = ("Error getting prompt nope\nTraceback (most recent call last):\n  File \"x.py\", line 1, in get_prompt\n"
             "    raise ValueError(f\"Unknown prompt: {name}\")\nValueError: Unknown prompt: nope\n")
    py = mcp_zone([answer("prompts/get", error={"code": 0, "message": "Unknown prompt: nope"})], trace)
    rs = mcp_zone([answer("prompts/get", error={"code": -32602, "message": "Unknown prompt: nope"})])
    assert rp.classify_zone(probe, py, rs) == ("readme", ["T047#7"])
    other_message = mcp_zone([answer("prompts/get", error={"code": -32602, "message": "Unknown prompt: other"})])
    assert rp.classify_zone(probe, py, other_message) == ("new", []), "the message must be equal"
    wrong_code = mcp_zone([answer("prompts/get", error={"code": -32601, "message": "Unknown prompt: nope"})])
    assert rp.classify_zone(probe, py, wrong_code) == ("new", []), "Rust's code must be -32602"
    assert rp.classify_zone(probe, mcp_zone(py["steps"], trace + "more\n"), rs) == ("new", []), "the stderr must be the traceback and nothing more"
    assert rp.classify_zone(probe, py, mcp_zone(rs["steps"], "something\n")) == ("new", []), "Rust must write nothing on stderr"


def test_protocol_rule_explains_only_the_three_named_differences():
    probe = {"explain": ["T047#8"]}
    caps_py = {"experimental": {}, "prompts": {"listChanged": False}, "resources": {"subscribe": False, "listChanged": False}, "tools": {"listChanged": False}}
    init = lambda caps: [answer("initialize", result={"capabilities": caps})]  # noqa: E731
    assert rp.classify_zone(probe, mcp_zone(init(caps_py)), mcp_zone(init({"prompts": {}, "tools": {}}))) == ("readme", ["T047#8"])
    assert rp.classify_zone(probe, mcp_zone(init(caps_py)), mcp_zone(init({"prompts": {}, "tools": {}, "logging": {}}))) == ("new", []), "one more capability is not the item"
    assert rp.classify_zone(probe, mcp_zone(init({**caps_py, "completions": {}})), mcp_zone(init({"prompts": {}, "tools": {}}))) == ("new", [])
    dump = "Failed to validate request: 2 validation errors for ClientRequest\nPingRequest.method\n  Input should be 'ping' [type=x]\n    For further information visit https://e/x\n" * 1
    dump += "InitializeRequest.method\n  Input should be 'initialize' [type=x]\n    For further information visit https://e/y\n"
    py_err = answer("nope/method", error={"code": -32602, "message": "Invalid request parameters", "data": ""})
    rs_err = answer("nope/method", error={"code": -32601, "message": "nope/method"})
    assert rp.classify_zone(probe, mcp_zone([py_err], dump), mcp_zone([rs_err])) == ("readme", ["T047#8"])
    assert rp.classify_zone(probe, mcp_zone([py_err], dump), mcp_zone([rs_err], "x\n")) == ("new", []), "Rust must write nothing on stderr"
    assert rp.classify_zone(probe, mcp_zone([py_err], dump + "extra\n"), mcp_zone([rs_err])) == ("new", []), "the dump must be the validation dump only"
    bad = answer("nope/method", error={"code": -32601, "message": "a sentence with spaces"})
    assert rp.classify_zone(probe, mcp_zone([py_err], dump), mcp_zone([bad])) == ("new", []), "Rust's message must be a method name"
    assert rp.classify_zone(probe, mcp_zone([py_err], dump), mcp_zone([answer("nope/method", error={"code": -32602, "message": "nope/method"})])) == ("new", [])
    log = "Processing request of type PingRequest\nProcessing request of type CallToolRequest\n"
    assert rp.classify_zone(probe, mcp_zone([], log), mcp_zone([])) == ("readme", ["T047#8"])
    assert rp.classify_zone(probe, mcp_zone([], log + "other\n"), mcp_zone([])) == ("new", [])


def test_write_only_route_rule_explains_only_404_against_405_with_the_page():
    probe = {"explain": ["T048#15"]}
    py = route_zone(404, "text/plain; charset=utf-8", "Not Found")
    assert rp.classify_zone(probe, py, route_zone(405, None, "")) == ("readme", ["T048#15"])
    assert rp.classify_zone(probe, py, route_zone(403, None, "")) == ("new", []), "only 405 is the item"
    assert rp.classify_zone(probe, py, route_zone(405, "application/json", "")) == ("new", [])
    assert rp.classify_zone(probe, py, route_zone(405, None, "x")) == ("new", [])
    assert rp.classify_zone(probe, route_zone(404, "text/plain; charset=utf-8", "Gone"), route_zone(405, None, "")) == ("new", [])


def test_each_t067_item_is_in_the_readme_marked_decided_for_you_and_has_a_rule():
    for rid in ("T058#13", "T058#14", "T051#8", "T047#7", "T047#8", "T048#15"):
        ref, number = rp.RULES[rid].ref
        body = rp.readme_item(ref, number)
        assert body, f"README `{ref}` item {number} is not there"
        assert "⚠ decided for you (t067)" in body.lower(), f"{ref} item {number} is not marked decided for you"
        assert rp.readme_ok(ref, number, rp.EXPLAIN_WORDS[(ref, number)]), f"{ref} item {number} lacks the words the rule states"
    assert rp.FINDINGS == {}, "the seven findings are closed: none is left in FINDINGS"


def test_the_report_has_no_new_difference_and_no_surface_left_unrun():
    text = report_text()
    found = rp.COUNTS.search(text)
    assert found and found.group(4) == "0" and found.group(5) == "0", f"differs new / not run is not 0: {found.group(0) if found else None}"
    assert not any(r["result"] == "differs: new" for r in rp.parse_report(text))
    assert "A SIGTERM to the runner skips" not in text, "the SIGTERM known limit must be gone"
    live = [r for r in rp.parse_report(text) if r["key"] == "route: GET /api/live"]
    assert live and live[0]["result"] == "equal", f"`/api/live` is aligned in Rust: it must be equal, not explained: {live}"


# ---- T067: SIGTERM and SIGHUP end a run the way Ctrl-C does: no server of the run alive, no world folder left

def _proc_table():
    import os

    table = {}
    for entry in os.listdir("/proc"):
        if entry.isdigit():
            try:
                raw = Path(f"/proc/{entry}/stat").read_text()
                rest = raw[raw.rindex(")") + 2:].split()
                table[int(entry)] = (int(rest[1]), rest[0], Path(f"/proc/{entry}/cmdline").read_text().replace("\0", " ")[:120])
            except (OSError, ValueError):
                pass
    return table


def _descendants(root):
    table, found, todo = _proc_table(), {}, [root]
    while todo:
        cur = todo.pop()
        for pid, (ppid, state, cmd) in table.items():
            if ppid == cur and state != "Z" and pid not in found:
                found[pid] = cmd
                todo.append(pid)
    return found


def _signal_run(args, signame, when, snippet=None):
    """Start the runner (`--only` through the command line, or `snippet` through `run_all`), send `signame` when `when(world_dirs, children)` holds.
    Returns (exit code, children seen before the signal, children alive afterwards, world folders left)."""
    import contextlib
    import os
    import signal
    import subprocess
    import tempfile
    import time

    with tempfile.TemporaryDirectory(prefix="t067-tmp-") as tmp:
        env = {"PATH": "/usr/bin:/bin", "HOME": os.environ.get("HOME", "/"), "PYTHONDONTWRITEBYTECODE": "1", "TMPDIR": tmp}
        cmd = [sys.executable, str(PARITY / "run_parity.py"), *args] if snippet is None else [sys.executable, "-c", snippet]
        proc = subprocess.Popen(cmd, env=env, cwd=str(PARITY.parent), stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
        seen, deadline = {}, time.time() + 60
        try:
            while proc.poll() is None and time.time() < deadline:
                kids = _descendants(proc.pid)
                seen.update(kids)
                worlds = [d for d in os.listdir(tmp) if d.startswith("parity-world-")]
                if when(worlds, kids):
                    break
                time.sleep(0.005)
            assert proc.poll() is None, f"the run ended before the moment to signal it: {proc.stdout.read()[-300:]!r} {proc.stderr.read()[-300:]!r}"
            proc.send_signal(getattr(signal, signame))
            code = proc.wait(timeout=60)
            time.sleep(0.5)
            table = _proc_table()
            alive = {pid: cmd for pid, cmd in seen.items() if pid in table and table[pid][1] != "Z"}
            left = sorted(os.listdir(tmp))
            return code, seen, alive, left
        finally:
            if proc.poll() is None:
                os.killpg(proc.pid, signal.SIGKILL)
            for pid in list(seen):  # a failed test must not leave its servers: only the PIDs this run started
                with contextlib.suppress(OSError):
                    os.kill(pid, signal.SIGKILL)
            for stream in (proc.stdout, proc.stderr):
                stream.close()


def _with_server(worlds, kids):
    return bool(kids)


def test_sigterm_during_a_running_server_kills_it_and_removes_the_world_folder():
    code, seen, alive, left = _signal_run(["--only", "cli: mcp / http-handshake-and-ping"], "SIGTERM", _with_server)
    assert seen, "no server of the run was seen: the test did not reach the state it tests"
    assert code == 128 + 15, f"exit code {code}"
    assert not alive, f"servers still alive: {alive}"
    assert left == [], f"left in the temp folder: {left}"


def test_sighup_during_a_running_server_kills_it_and_removes_the_world_folder():
    code, seen, alive, left = _signal_run(["--only", "cli: mcp / http-handshake-and-ping"], "SIGHUP", _with_server)
    assert seen and code == 128 + 1 and not alive and left == [], (code, seen, alive, left)


def test_sigterm_while_a_serve_starts_kills_it():
    """`Server.__init__` waits for the URL line; the caller never gets the object, so the constructor itself must close the child."""
    code, seen, alive, left = _signal_run(["--only", "route: GET /api/live / first-event"], "SIGTERM", _with_server)
    assert seen and code == 128 + 15 and not alive and left == [], (code, seen, alive, left)


def test_sigterm_before_any_server_exists_removes_the_world_folder():
    """The world folder exists (the store copy and the fixtures are being made) and no child has started yet."""
    code, seen, alive, left = _signal_run(["--only", "cli: mcp / http-handshake-and-ping"], "SIGTERM", lambda worlds, kids: bool(worlds) and not kids)
    assert code == 128 + 15 and not alive and left == [], (code, seen, alive, left)


def test_sigterm_in_the_full_run_path_cleans_up_as_well():
    """`run_all` (what a plain run and `--check` use) has its own `finally`: signal it through a driver that calls it with one probe."""
    snippet = ("import sys; sys.path.insert(0, 'parity'); import run_parity as rp; rp.stop_on_signals()\n"
               "try:\n    rp.run_all(['cli: mcp / http-handshake-and-ping'])\nexcept rp.Terminated as stop:\n    sys.exit(128 + stop.signum)\n")
    code, seen, alive, left = _signal_run([], "SIGTERM", _with_server, snippet=snippet)
    assert seen and code == 128 + 15 and not alive and left == [], (code, seen, alive, left)


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
